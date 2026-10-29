import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { fixture, node } from "./fixtures.mjs";

const RED = [255, 0, 0, 255];
const WHITE = [255, 255, 255, 255];
const CLEAR = [0, 0, 0, 0];
let worker;
before(async () => {
  worker = await PipelineWorker.start();
});
after(async () => {
  await worker?.close();
});

/** Root frame without fill (transparent) holding the given nodes; frame-local pixels. */
async function paint(children) {
  const input = fixture(160);
  input.viewport.height = 100;
  input.assets = [];
  input.nodes = [node("frame", null, "frame", { layout: { mode: "free" } }), ...children];
  const { scene } = await worker.compute(input);
  const png = await worker.render(scene);
  const at = (points) =>
    worker.runtime.page.evaluate((points) => {
      const c = document.getElementById("scene").getContext("2d");
      return points.map(([x, y]) => Array.from(c.getImageData(x, y, 1, 1).data));
    }, points);
  return { scene, png, at };
}

async function expectPixels(children, expected) {
  const { at } = await paint(children);
  const points = expected.flatMap(([, list]) => list);
  const colors = await at(points);
  let n = 0;
  for (const [color, list] of expected)
    for (const p of list) assert.deepEqual(colors[n++], color, `pixel ${p}`);
}

const card = (stroke, extra = {}) =>
  node("card", "frame", "rectangle", {
    x: 10, y: 10, width: 100, height: 50, radius: 8, fill: "#ffffff", stroke, ...extra,
  });

test("AC6 inside stroke stays within bounds and keeps the rounded corner", async () => {
  await expectPixels([card({ color: "#ff0000", width: 2 })], [
    [RED, [[50, 10], [50, 11], [10, 35], [11, 35], [109, 35], [50, 59]]],
    [WHITE, [[50, 12], [12, 35]]],
    [CLEAR, [[9, 35], [110, 35], [50, 9], [50, 60], [10, 10], [11, 11]]],
  ]);
});

test("AC7 center and outside strokes straddle or surround the bounds", async () => {
  await expectPixels([card({ color: "#ff0000", width: 2, align: "center" })], [
    [RED, [[50, 9], [50, 10]]],
    [CLEAR, [[50, 8]]],
    [WHITE, [[50, 11]]],
  ]);
  await expectPixels([card({ color: "#ff0000", width: 2, align: "outside" })], [
    [RED, [[50, 8], [50, 9], [8, 35], [9, 35]]],
    [CLEAR, [[50, 7], [7, 35], [8, 8]]],
    [WHITE, [[50, 10], [10, 35]]],
  ]);
});

const keycap = (color, fill = "#ffffff") =>
  node("key", "frame", "rectangle", {
    x: 10, y: 10, width: 40, height: 20, radius: 5,
    ...(fill ? { fill } : {}), stroke: { color, width: [1, 1, 2, 1] },
  });

test("AC8 per-side colors keep the corner rounded (keycap)", async () => {
  await expectPixels([keycap(["#d0d5dd", "#d0d5dd", "#667085", "#d0d5dd"])], [
    [[208, 213, 221, 255], [[30, 10], [10, 20], [49, 20]]],
    [[102, 112, 133, 255], [[30, 28], [30, 29]]],
    [WHITE, [[30, 11], [30, 27]]],
    [CLEAR, [[10, 10], [30, 30]]],
  ]);
});

test("AC9 bottom-only stroke and translucent side colors keep exact alpha", async () => {
  const bottom = node("line", "frame", "rectangle", {
    x: 10, y: 10, width: 100, height: 40, fill: "#ffffff",
    stroke: { color: "#000000", width: [0, 0, 1, 0] },
  });
  await expectPixels([bottom], [
    [[0, 0, 0, 255], [[60, 49]]],
    [WHITE, [[60, 48], [10, 30]]],
    [CLEAR, [[60, 50]]],
  ]);
  await expectPixels([keycap(["#00000040", "#00000040", "#00000080", "#00000040"], null)], [
    [[0, 0, 0, 128], [[30, 28]]],
    [[0, 0, 0, 64], [[30, 10]]],
  ]);
});

test("AC11 a frame's stroke paints above its children and outside its own clip", async () => {
  const frame = (stroke) => [
    node("clipper", "frame", "frame", {
      x: 10, y: 10, width: 100, height: 50, clip: true, layout: { mode: "free" }, stroke,
    }),
    node("blue", "clipper", "rectangle", { x: 0, y: 0, width: 100, height: 50, fill: "#2563eb" }),
  ];
  await expectPixels(frame({ color: "#ff0000", width: 2 }), [
    [RED, [[50, 10]]],
    [[37, 99, 235, 255], [[50, 30]]],
  ]);
  await expectPixels(frame({ color: "#ff0000", width: 2, align: "outside" }), [
    [RED, [[50, 8]]],
  ]);
});

test("AC12 viewer canvas and PNG are byte-identical with per-side strokes; old scenes are refused", async () => {
  const { scene, png } = await paint([
    keycap(["#d0d5dd", "#d0d5dd", "#667085", "#d0d5dd"]),
    card({ color: ["#ff000080", "#00ff00", "#0000ff", "#000"], width: [1, 2, 3, 4], align: "center" }),
  ]);
  assert.deepEqual(scene.nodes.find((n) => n.id === "key").stroke.width, [1, 1, 2, 1]);
  const viewer = await worker.runtime.page.evaluate((scene) => {
    const canvas = document.createElement("canvas");
    renderer.drawScene(canvas, scene, resources);
    let mismatch = null;
    try {
      const old = structuredClone(scene);
      old.provenance.sceneVersion = "scene-1";
      renderer.drawScene(document.createElement("canvas"), old, resources);
    } catch (e) {
      mismatch = e.code;
    }
    return { base64: canvas.toDataURL("image/png").split(",")[1], mismatch };
  }, scene);
  assert.equal(viewer.base64, png.base64, "zero pixel tolerance on pinned runtime");
  assert.equal(viewer.mismatch, "PIPELINE_MISMATCH");
});
