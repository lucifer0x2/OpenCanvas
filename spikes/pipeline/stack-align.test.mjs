import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { fixture, node } from "./fixtures.mjs";
let worker;
before(async () => {
  worker = await PipelineWorker.start();
});
after(async () => {
  await worker?.close();
});

// P08a: per-side padding, align and justify through the real Chromium pipeline.
function centered(width, height) {
  const input = fixture(width);
  input.viewport.height = height;
  input.assets = [];
  input.nodes = [
    node("frame", null, "frame", {
      width,
      height,
      fill: "#ffffff",
      layout: { mode: "column", padding: [8, 16, 24, 32], align: "center", justify: "center" },
    }),
    node("card", "frame", "group", {
      width: "hug",
      height: "hug",
      fill: "#2563eb",
      layout: { mode: "row", padding: [4, 8], gap: 6, align: "end", justify: "space-between" },
    }),
    node("red", "card", "rectangle", { width: 20, height: 10, fill: "#ef4444" }),
    node("green", "card", "rectangle", { width: 30, height: 30, fill: "#22c55e" }),
    node("title", "frame", "text", {
      text: "Căn giữa",
      fontId: "font_bundled",
      fontSize: 16,
      lineHeight: 24,
    }),
  ];
  return input;
}
const bounds = (scene, id) => scene.nodes.find((n) => n.id === id).bounds;

test("centered stack with per-side padding places and paints boxes like CSS flexbox", async () => {
  const scene = (await worker.compute(centered(400, 300))).scene;
  const card = bounds(scene, "card");
  assert.deepEqual([card.x, card.y, card.width, card.height], [172, 111, 72, 38]);
  assert.deepEqual([bounds(scene, "red").x, bounds(scene, "red").y], [180, 135]);
  assert.deepEqual([bounds(scene, "green").x, bounds(scene, "green").y], [206, 115]);
  const title = bounds(scene, "title");
  assert.ok(Math.abs(title.x + title.width / 2 - 208) <= 1 / 64, JSON.stringify(title));
  await worker.render(scene);
  const colors = await worker.runtime.page.evaluate(() => {
    const c = document.getElementById("scene").getContext("2d");
    return [
      [173, 112],
      [181, 136],
      [207, 116],
      [171, 112],
    ].map(([x, y]) => Array.from(c.getImageData(x, y, 1, 1).data));
  });
  assert.deepEqual(colors, [
    [37, 99, 235, 255],
    [239, 68, 68, 255],
    [34, 197, 94, 255],
    [255, 255, 255, 255],
  ]);
  const narrow = (await worker.compute(centered(320, 200))).scene;
  assert.deepEqual([bounds(narrow, "card").x, bounds(narrow, "card").y], [132, 61]);
});

test("text laid out at its own max-content width keeps one line", async () => {
  const input = centered(1280, 800);
  const samples = [
    ["Chào mừng đến QueryNest", 28, 42],
    ["QueryNest / Welcome", 14, 21],
    ["⌘C", 14, 20],
    ["Căn giữa", 16, 24],
  ];
  input.nodes = input.nodes.slice(0, 1);
  samples.forEach(([text, fontSize, lineHeight], i) =>
    input.nodes.push(
      node(`t${i}`, "frame", "text", { text, fontId: "font_bundled", fontSize, lineHeight }),
    ),
  );
  const scene = (await worker.compute(input)).scene;
  for (const [i, [text, , lineHeight]] of samples.entries()) {
    const t = scene.nodes.find((n) => n.id === `t${i}`);
    assert.deepEqual(t.lines.map((l) => l.text), [text]);
    assert.equal(t.bounds.height, lineHeight);
  }
});
