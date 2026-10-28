// Group opacity composites like CSS: the subtree renders as one image, then fades once.
import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { fixture, imageSVG } from "./fixtures.mjs";
import { cssPixels } from "./css-oracle.mjs";
import {
  box,
  clipped,
  cssCases,
  frame,
  group,
  keycap,
  nested,
  overlap,
} from "./opacity-fixtures.mjs";

let worker;
before(async () => {
  worker = await PipelineWorker.start({
    resourceHandler: async (req, res) => {
      if (req.url !== "/fixture.svg") return false;
      res.writeHead(200, { "content-type": "image/svg+xml" });
      res.end(imageSVG);
      return true;
    },
  });
});
after(async () => {
  await worker?.close();
});

function input(nodes, width = 200, height = 120) {
  const f = fixture(width);
  f.viewport.height = height;
  f.nodes = nodes;
  return f;
}

/** Renders through the worker and reads RGBA at each point of the worker canvas. */
async function render(nodes, points, scale = 1) {
  const scene = (await worker.compute(input(nodes))).scene;
  const { draw } = await worker.render(scene, { scale });
  const pixels = await worker.runtime.page.evaluate((points) => {
    const c = document.getElementById("scene").getContext("2d");
    return points.map(([x, y]) => Array.from(c.getImageData(x, y, 1, 1).data));
  }, points);
  return { draw, pixels, scene };
}

const near = (actual, expected, tolerance, label) =>
  actual.forEach((v, i) =>
    assert.ok(
      Math.abs(v - expected[i]) <= tolerance,
      `${label}: ${actual} vs ${expected}`,
    ),
  );

test("AC1 the overlap inside a faded group matches the non-overlapping part", async () => {
  const { draw, pixels } = await render(overlap(), [
    [60, 40],
    [90, 40],
  ]);
  near(pixels[0], pixels[1], 1, "overlap vs blue only");
  assert.equal(draw.opacityLayers, 1);
  // Folding 0.5 into each child (the old renderer) showed red through blue: 142,130,198.
});

test("AC2 AC4 renderer pixels match Chrome CSS opacity, nesting and rounded clipping", async () => {
  for (const [nodes, html, points] of cssCases()) {
    const { pixels } = await render(nodes, points);
    const css = await cssPixels(worker.runtime.browser, html, 200, 120, points);
    pixels.forEach((p, i) => near(p, css[i], 2, `point ${points[i]}`));
  }
  const { pixels } = await render(clipped(), [
    [21, 21],
    [110, 60],
  ]);
  for (const p of pixels) assert.deepEqual(p, [255, 255, 255, 255]);
});

test("AC3 nested group opacity multiplies once per group", async () => {
  const { draw, pixels } = await render(nested(), [
    [80, 60],
    [80, 30],
    [50, 30],
  ]);
  near(pixels[0], [200, 216, 250, 255], 1, "blue at 0.25 on white");
  assert.ok(pixels[1].some((v, i) => Math.abs(v - pixels[2][i]) > 20));
  assert.equal(draw.opacityLayers, 2);
});

test("AC5 scenes without multi-paint opacity groups use no layer", async () => {
  for (const scale of [1, 2]) {
    const scene = (await worker.compute(fixture(420))).scene;
    assert.equal((await worker.render(scene, { scale })).draw.opacityLayers, 0);
  }
  const leaf = fixture(420);
  leaf.nodes[3].props.opacity = 0.5;
  leaf.nodes[1].props.opacity = 0.7;
  const scene = (await worker.compute(leaf)).scene;
  assert.equal((await worker.render(scene)).draw.opacityLayers, 0);
  // A group with a single painting child folds its alpha: no layer either.
  const single = [
    frame(),
    group("g", "frame", 0, 0, 50, 50, { opacity: 0.5 }),
    box("r", "g", 0, 0, 20, 20, { fill: "#000000" }),
  ];
  assert.equal((await render(single, [[5, 5]])).draw.opacityLayers, 0);
});

test("AC6 a fully transparent group paints nothing", async () => {
  const points = [
    [30, 40],
    [60, 40],
    [90, 40],
    [5, 5],
  ];
  const hidden = overlap();
  hidden[1].props.opacity = 0;
  hidden.push(box("third", "g", 80, 0, 20, 20, { fill: "#000000" }));
  const a = await render(hidden, points);
  const b = await render([frame()], points);
  assert.deepEqual(a.pixels, b.pixels);
  assert.equal(a.draw.opacityLayers, 0);
});

test("AC7 over the layer budget the group folds its alpha like before, without throwing", async () => {
  const { scene } = await render(nested(), [[0, 0]]);
  const result = await worker.runtime.page.evaluate((scene) => {
    const canvas = document.createElement("canvas");
    canvas.width = 200;
    canvas.height = 120;
    const ctx = canvas.getContext("2d");
    const stats = renderer.paintTree(
      ctx,
      scene,
      resources,
      renderer.paintNode,
      { maxLayerPixels: 5000 },
    );
    const px = ([x, y]) => Array.from(ctx.getImageData(x, y, 1, 1).data);
    return {
      stats,
      pixels: [
        [50, 30],
        [80, 30],
        [80, 60],
      ].map(px),
    };
  }, scene);
  assert.deepEqual(result.stats, {
    opacityLayers: 0,
    opacityLayerFallbacks: 2,
  });
  // Over the transparent canvas the folded paints are premultiplied; compare against white.
  const onWhite = (p) =>
    p
      .slice(0, 3)
      .map((v) => Math.round(v * (p[3] / 255) + 255 * (1 - p[3] / 255)));
  const expected = [
    [163, 173, 140],
    [131, 154, 164],
    [197, 181, 215],
  ];
  result.pixels.forEach((p, i) =>
    near(onWhite(p), expected[i], 2, `folded ${i}`),
  );
});

test("AC8 the note #10 keycap shows its background, not the borders under it", async () => {
  const nodes = keycap();
  const { draw, pixels } = await render(nodes, [
    [24, 26],
    [56, 26],
  ]);
  near(pixels[0], [249, 251, 253, 255], 1, "background @0.8");
  assert.equal(draw.opacityLayers, 1);
  // The label renders inside the layer: some pixel in its box is darker than the background.
  const ink = await worker.runtime.page.evaluate(() => {
    const d = document
      .getElementById("scene")
      .getContext("2d")
      .getImageData(28, 22, 20, 16).data;
    return Math.min(...d.filter((_, i) => i % 4 === 0));
  });
  assert.ok(ink < 200, `label ink ${ink}`);
});
