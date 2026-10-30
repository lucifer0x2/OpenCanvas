// Run: node --test renderer/text-extent.test.mjs. scene-tree.ts uses extensionless
// bundler imports, so it is bundled in memory instead of loaded by Node type stripping.
import assert from "node:assert/strict";
import { test } from "node:test";
import { build } from "esbuild";

const bundle = await build({ entryPoints: [new URL("./scene-tree.ts", import.meta.url).pathname], bundle: true, format: "esm", write: false });
const { indexScene } = await import(`data:text/javascript;base64,${Buffer.from(bundle.outputFiles[0].text).toString("base64")}`);

const textNode = (inkBounds) => ({ id: "text", parentId: null, kind: "text", fill: "#000", fontId: "f", fontSize: 14, lineHeight: 20,
  bounds: { x: 100, y: 10, width: 20, height: 20 }, lines: [{ text: "Wide", x: 0, y: 15, width: 0, ...(inkBounds && { inkBounds }) }] });
const resources = { fonts: new Map([["f", {}]]), images: new Map() };

test("cached text ink overhang expands opacity-layer extent beyond advance width", () => {
  const tree = indexScene({ nodes: [textNode({ x: -400, y: -11, width: 410, height: 14 })] }, resources);
  const extent = tree.extent.get("text");
  assert.equal(extent.x0, -300);
  assert.ok(extent.x1 >= 110);
});

test("ink below and right of the line box also widens the extent", () => {
  const tree = indexScene({ nodes: [textNode({ x: 0, y: -11, width: 600, height: 400 })] }, resources);
  const extent = tree.extent.get("text");
  assert.equal(extent.x1, 700);
  assert.equal(extent.y1, 10 + 15 - 11 + 400);
});

test("lines without inkBounds keep the font-size padded fallback extent", () => {
  const tree = indexScene({ nodes: [textNode()] }, resources);
  assert.deepEqual(tree.extent.get("text"), { x0: 80, y0: -10, x1: 140, y1: 50 });
});

test("a group's extent includes child ink overhang so the opacity layer covers it", () => {
  const group = { id: "g", parentId: null, kind: "group", fill: "transparent", fontSize: 0, lineHeight: 0, opacity: 0.5,
    bounds: { x: 100, y: 10, width: 20, height: 20 }, lines: [] };
  const child = { ...textNode({ x: -400, y: -11, width: 410, height: 14 }), parentId: "g" };
  const tree = indexScene({ nodes: [group, child] }, resources);
  assert.equal(tree.extent.get("g").x0, -300);
});
