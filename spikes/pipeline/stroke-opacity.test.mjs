// Stroke under group opacity: the stroke is part of the node's layer, and a layer grows to hold
// strokes painted past bounds (outside/center), compared with Chrome's CSS border/outline.
import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { fixture } from "./fixtures.mjs";
import { cssPixels, div } from "./css-oracle.mjs";
import { box, frame, group } from "./opacity-fixtures.mjs";

let worker;
before(async () => {
  worker = await PipelineWorker.start();
});
after(async () => {
  await worker?.close();
});

async function render(nodes, points) {
  const input = fixture(200);
  input.viewport.height = 120;
  input.assets = [];
  input.nodes = nodes;
  const { scene } = await worker.compute(input);
  const { draw } = await worker.render(scene);
  const pixels = await worker.runtime.page.evaluate((points) => {
    const c = document.getElementById("scene").getContext("2d");
    return points.map(([x, y]) => Array.from(c.getImageData(x, y, 1, 1).data));
  }, points);
  return { draw, pixels };
}

async function matchesCSS(nodes, html, points) {
  const { draw, pixels } = await render(nodes, points);
  const css = await cssPixels(worker.runtime.browser, html, 200, 120, points);
  pixels.forEach((p, i) => near(p, css[i], `point ${points[i]}:`));
  return draw;
}

const BLUE = "#2563eb";
const RED = "#ff0000";

const HALF_RED = [255, 128, 128, 255];
const near = (actual, expected, label = "") =>
  actual.forEach((v, c) =>
    assert.ok(
      Math.abs(v - expected[c]) <= 2,
      `${label} ${actual} vs ${expected}`,
    ),
  );

test("a faded box composites fill and inside stroke once, like a CSS border", async () => {
  const nodes = [
    frame(),
    box("b", "frame", 20, 20, 100, 50, {
      fill: BLUE,
      opacity: 0.5,
      stroke: { color: RED, width: 6 },
    }),
  ];
  const html = div(
    20,
    20,
    100,
    50,
    `background:${BLUE};border:6px solid ${RED};opacity:.5`,
  );
  const draw = await matchesCSS(nodes, html, [
    [22, 45],
    [70, 22],
    [70, 45],
  ]);
  assert.equal(draw.opacityLayers, 1);
});

test("the layer grows to hold an outside stroke past the node's bounds", async () => {
  const nodes = [
    frame(),
    box("b", "frame", 20, 20, 100, 50, {
      fill: BLUE,
      opacity: 0.5,
      stroke: { color: RED, width: 8, align: "outside" },
    }),
  ];
  const html = div(
    20,
    20,
    100,
    50,
    `background:${BLUE};outline:8px solid ${RED};opacity:.5`,
  );
  await matchesCSS(nodes, html, [
    [13, 45],
    [70, 13],
    [125, 45],
    [70, 76],
    [70, 45],
  ]);
});

test("a faded group's layer holds a child's outside stroke past the group bounds", async () => {
  const nodes = [
    frame(),
    group("g", "frame", 30, 30, 80, 40, { opacity: 0.5 }),
    box("a", "g", 0, 0, 80, 40, {
      fill: BLUE,
      stroke: { color: RED, width: 10, align: "outside" },
    }),
    box("c", "g", 60, 20, 40, 40, { fill: "#16a34a" }),
  ];
  const { draw, pixels } = await render(nodes, [
    [22, 50],
    [70, 22],
  ]);
  assert.equal(draw.opacityLayers, 1);
  // Red at 0.5 over white, not cut by a layer sized to the group's own bounds.
  for (const p of pixels) near(p, HALF_RED);
});

test("a clipping node's own outside stroke survives its clip under opacity", async () => {
  const nodes = [
    frame(),
    group("g", "frame", 30, 30, 80, 40, {
      opacity: 0.5,
      clip: true,
      fill: BLUE,
      stroke: { color: RED, width: 6, align: "outside" },
    }),
    box("inner", "g", -20, 10, 40, 20, { fill: "#16a34a" }),
  ];
  const { pixels } = await render(nodes, [
    [26, 50],
    [20, 50],
  ]);
  near(pixels[0], HALF_RED);
  assert.deepEqual(pixels[1], [255, 255, 255, 255]);
});
