import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { fixture, imageSVG, node } from "./fixtures.mjs";
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
test("paint order, frame-local coordinates, nested clipping and source crop are honored", async () => {
  const input = fixture(120);
  input.viewport.height = 120;
  input.nodes = [
    node("frame", null, "frame", { fill: "#ffffff", layout: { mode: "free" } }),
    node("group", "frame", "group", {
      x: 10,
      y: 10,
      width: 40,
      height: 40,
      clip: true,
      layout: { mode: "free" },
    }),
    node("under", "group", "rectangle", {
      x: 0,
      y: 0,
      width: 80,
      height: 80,
      fill: "#ef4444",
    }),
    node("over", "group", "rectangle", {
      x: 15,
      y: 15,
      width: 20,
      height: 20,
      fill: "#22c55e",
    }),
    node("picture", "frame", "image", {
      x: 60,
      y: 10,
      width: 40,
      height: 40,
      assetId: "image",
      fit: "fill",
      crop: { x: 0.5, y: 0, width: 0.5, height: 1 },
    }),
  ];
  const scene = (await worker.compute(input)).scene;
  assert.deepEqual(
    scene.nodes.map((n) => n.id),
    ["frame", "group", "under", "over", "picture"],
  );
  assert.equal(scene.nodes.find((n) => n.id === "over").bounds.x, 25);
  await worker.render(scene);
  const colors = await worker.runtime.page.evaluate(() => {
    const c = document.getElementById("scene").getContext("2d");
    return [
      [15, 15],
      [30, 30],
      [55, 15],
      [75, 20],
    ].map(([x, y]) => Array.from(c.getImageData(x, y, 1, 1).data));
  });
  assert.deepEqual(colors, [
    [239, 68, 68, 255],
    [34, 197, 94, 255],
    [255, 255, 255, 255],
    [245, 158, 11, 255],
  ]);
});
