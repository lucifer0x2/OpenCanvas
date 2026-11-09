// Run: node --test renderer/viewport-paint.test.mjs
import assert from "node:assert/strict";
import { test } from "node:test";
import { build } from "esbuild";

const bundle = await build({
  stdin: {
    contents: 'export { paintTree } from "./layers"; export { paintScene } from "./paint"; export { indexScene } from "./scene-tree";',
    resolveDir: new URL(".", import.meta.url).pathname,
  },
  bundle: true, format: "esm", write: false,
});
const { paintTree, paintScene, indexScene } = await import(
  `data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString("base64")}`
);

const identity = { a: 1, b: 0, c: 0, d: 1, e: 0, f: 0 };
const resources = () => ({ fonts: new Map([["f", {}]]), images: new Map([["i", {}]]) });
const node = (id, x = 10, extra = {}) => ({
  id, parentId: null, kind: "rectangle", opacity: 1, fill: "#000", clip: false,
  bounds: { x, y: 10, width: 10, height: 10 }, lines: [], fontId: "f",
  assetId: "i", fontSize: 10, lineHeight: 12, radius: 0, ...extra,
});

function context(transform = identity, width = 100, height = 100) {
  let matrix = { ...transform };
  const stack = [];
  return {
    canvas: { width, height }, globalAlpha: 1, reads: 0, fills: 0, composites: [],
    getTransform() { this.reads++; return { ...matrix }; },
    setTransform(a, b, c, d, e, f) { matrix = { a, b, c, d, e, f }; },
    save() { stack.push([matrix, this.globalAlpha]); },
    restore() { [matrix, this.globalAlpha] = stack.pop(); },
    beginPath() {}, roundRect() {}, clip() {},
    fill() { this.fills++; },
    drawImage(canvas, x, y) { this.composites.push([canvas.width, canvas.height, x, y]); },
  };
}

function painted(nodes, transform = identity, options = {}, loaded = resources()) {
  const ctx = context(transform);
  const calls = [];
  const stats = paintTree(ctx, { nodes }, loaded, (c, n) => calls.push([n.id, c.globalAlpha]), options);
  return { ctx, calls, stats };
}
const ids = (result) => result.calls.map(([id]) => id);

test("prepared tree is reused on pan/zoom without accessing scene nodes again", () => {
  let reads = 0;
  const scene = { get nodes() { reads++; return [node("visible")]; } };
  const loaded = resources();
  const tree = indexScene(scene, loaded);
  const indexedReads = reads;
  for (const transform of [identity, { ...identity, e: -1 }, { ...identity, a: 0.2, d: 0.2 }]) {
    const ctx = context(transform);
    paintScene(ctx, scene, loaded, tree);
    assert.equal(ctx.fills, 1);
  }
  assert.equal(reads, indexedReads);
  paintScene(context(), scene, loaded);
  assert.ok(reads > indexedReads, "callers without a prepared tree still index normally");
});

test("opaque offscreen subtrees issue no paint commands; siblings share one transform read", () => {
  const result = painted([
    node("offscreen", 500, { fill: "transparent" }),
    node("offscreen-child", 500, { parentId: "offscreen" }),
    node("visible"), node("also-visible", 30),
  ]);
  assert.deepEqual(ids(result), ["visible", "also-visible"]);
  assert.equal(result.ctx.reads, 1);
});

test("prepared and default paths agree under scale, pan and rotated transforms", () => {
  const nodes = [node("near"), node("far", 600)];
  const loaded = resources();
  const tree = indexScene({ nodes }, loaded);
  const cases = [
    [{ ...identity, a: 0.1, d: 0.1 }, ["near", "far"]],
    [{ ...identity, a: 0.2, d: 0.2 }, ["near"]],
    [{ ...identity, e: -550 }, ["far"]],
    [{ a: 0, b: 1, c: -1, d: 0, e: 30, f: 0 }, ["near"]],
  ];
  for (const [transform, expected] of cases) {
    const direct = painted(nodes, transform, {}, loaded);
    const prepared = painted(nodes, transform, { tree }, loaded);
    assert.deepEqual(ids(direct), expected);
    assert.deepEqual(prepared.calls, direct.calls);
  }
});

test("one device pixel AA margin survives at the viewport edge", () => {
  assert.deepEqual(ids(painted([node("edge", 100.2)])), ["edge"]);
  assert.deepEqual(ids(painted([node("outside", 102)])), []);
});

test("overflow descendants, outside strokes and outer shadows remain visible", () => {
  const result = painted([
    node("overflow-parent", 200, { fill: "transparent" }),
    node("overflow-child", 20, { parentId: "overflow-parent" }),
    node("stroke", 110, { stroke: { align: "outside", width: [20, 20, 20, 20], color: ["#000", "#000", "#000", "#000"] } }),
    node("shadow", 200, { shadow: [{ x: -150, y: 0, blur: 4, spread: 0, inset: false, color: "#000" }] }),
  ]);
  assert.deepEqual(ids(result), ["overflow-parent", "overflow-child", "stroke", "shadow"]);
});

test("text ink overhang and decoration are included beyond authored boxes", () => {
  const text = (id, extra) => node(id, 500, {
    kind: "text", lines: [{ text: "ink", x: 0, y: 15, width: 20,
      inkBounds: { x: -480, y: -10, width: 490, height: 14 } }], ...extra,
  });
  const decorated = text("decoration", {
    bounds: { x: 10, y: -500, width: 10, height: 10 },
    lines: [{ text: "ink", x: 0, y: 15, width: 20 }],
    decoration: { line: "underline", offset: 500, thickness: 3 },
  });
  assert.deepEqual(ids(painted([text("overhang"), decorated])), ["overhang", "decoration"]);
});

test("an inset shadow on a transparent node still paints within its own box", () => {
  const shadow = { x: 0, y: 0, blur: 4, spread: 0, inset: true, color: "#000" };
  const nodes = [node("inset", 10, { fill: "transparent", shadow: [shadow] }),
    node("offscreen-inset", 500, { fill: "transparent", shadow: [shadow] })];
  const loaded = resources();
  const tree = indexScene({ nodes }, loaded);
  assert.deepEqual(tree.extent.get("inset"), { x0: 10, y0: 10, x1: 20, y1: 20 });
  assert.deepEqual(ids(painted(nodes)), ["inset"]);
  assert.deepEqual(ids(painted(nodes, identity, { tree }, loaded)), ["inset"]);
});

test("missing font/image placeholders and unknown prepared extents are conservative", () => {
  const loaded = { fonts: new Map(), images: new Map() };
  const nodes = [node("font", 500, { kind: "text" }), node("image", 500, { kind: "image" })];
  assert.deepEqual(ids(painted(nodes, identity, {}, loaded)), ["font", "image"]);
  const tree = indexScene({ nodes: [node("unknown", 500)] }, resources());
  tree.extent.delete("unknown");
  assert.deepEqual(ids(painted([node("unknown", 500)], identity, { tree })), ["unknown"]);
  // Retry publishes a new ready-resource object even if its maps are shared by board slots.
  const failedTree = indexScene({ nodes }, loaded);
  const ready = { ...loaded };
  ready.fonts.set("f", {}); ready.images.set("i", {});
  const readyTree = indexScene({ nodes }, ready);
  assert.deepEqual(ids(painted(nodes, identity, { tree: failedTree }, loaded)), ["font", "image"]);
  assert.deepEqual(ids(painted(nodes, identity, { tree: readyTree }, ready)), []);
});

test("own clip rejects children but preserves an outside stroke", () => {
  const result = painted([
    node("clipped", 110, { clip: true, fill: "transparent",
      stroke: { align: "outside", width: [20, 20, 20, 20], color: ["#000", "#000", "#000", "#000"] } }),
    node("hidden-child", 20, { parentId: "clipped" }),
    node("visible-parent", 10, { clip: true }),
    node("outside-clip", 60, { parentId: "visible-parent" }),
  ], identity, { paintOver: (c, n) => { if (n.stroke) c.fills++; } });
  assert.deepEqual(ids(result), ["clipped", "visible-parent"]);
  assert.equal(result.ctx.fills, 1);
});

test("group opacity uses the same visible subtree extent for prepared and default paths", () => {
  const previous = globalThis.document;
  globalThis.document = { createElement: () => {
    const canvas = { width: 0, height: 0 };
    const ctx = context(); ctx.canvas = canvas;
    canvas.getContext = () => ctx;
    return canvas;
  } };
  try {
    const nodes = [node("group", 200, { opacity: 0.5 }), node("overflow", 20, { parentId: "group" })];
    const loaded = resources();
    const tree = indexScene({ nodes }, loaded);
    const direct = painted(nodes, identity, {}, loaded);
    const prepared = painted(nodes, identity, { tree }, loaded);
    assert.equal(direct.stats.opacityLayers, 1);
    assert.deepEqual(direct.calls, [["group", 1], ["overflow", 1]]);
    assert.deepEqual(prepared.calls, direct.calls);
    assert.deepEqual(prepared.ctx.composites, direct.ctx.composites);
    assert.deepEqual(prepared.stats, direct.stats);
  } finally { globalThis.document = previous; }
});

test("visible fill/child/stroke order and opacity fallback stay identical with a prepared tree", () => {
  const nodes = [
    node("group", 10, { opacity: 0.5, clip: true,
      bounds: { x: 10, y: 10, width: 50, height: 50 } }),
    node("child", 20, { parentId: "group" }),
    node("clipped-child", 90, { parentId: "group" }),
    node("sibling", 70),
  ];
  const loaded = resources();
  const tree = indexScene({ nodes }, loaded);
  const run = (prepared) => {
    const calls = [];
    const stats = paintTree(context({ ...identity, a: 0.5, d: 0.5 }), { nodes }, loaded,
      (c, n) => calls.push(["own", n.id, c.globalAlpha]), {
        maxLayerPixels: 0, ...(prepared && { tree }),
        paintOver: (c, n) => calls.push(["over", n.id, c.globalAlpha]),
      });
    return { calls, stats };
  };
  const direct = run(false);
  assert.deepEqual(direct.calls, [
    ["own", "group", 0.5], ["own", "child", 0.5], ["over", "child", 0.5],
    ["over", "group", 0.5], ["own", "sibling", 1], ["over", "sibling", 1],
  ]);
  assert.equal(direct.stats.opacityLayerFallbacks, 1);
  assert.deepEqual(run(true), direct);
});

test("custom own painters can transform descendants without leaking to siblings", () => {
  const ctx = context();
  const calls = [];
  paintTree(ctx, { nodes: [
    node("parent", 10), node("child", 200, { parentId: "parent" }), node("sibling", 200),
  ] }, resources(), (c, n) => {
    calls.push(n.id);
    if (n.id === "parent") c.setTransform(1, 0, 0, 1, -190, 0);
  });
  assert.deepEqual(calls, ["parent", "child"]);
  assert.equal(ctx.reads, 2);
});
