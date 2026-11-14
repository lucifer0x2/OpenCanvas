import assert from "node:assert/strict";
import test from "node:test";
import { sceneModule } from "./scene-module.mjs";

const { packFrames, fitFrameBoard, FRAME_LABEL, FRAME_GAP } =
  await sceneModule("frameLayout");
const { frameLabelMetrics } = await sceneModule("frameLabels");
const { boardHitTest } = await sceneModule("boardHitTest");
const dimensions = (count, width = 800, height = 600) =>
  Array.from({ length: count }, (_, id) => ({ id: String(id), width, height }));

function assertPacked(input, aspect) {
  const layout = packFrames(input, aspect);
  assert.deepEqual(
    layout.frames.map((frame) => frame.id),
    input.map((frame) => frame.id),
  );
  for (const frame of layout.frames) {
    assert.ok(frame.x >= 0 && frame.y >= FRAME_LABEL);
    assert.ok(frame.x + frame.width <= layout.width + 0.001);
    assert.ok(frame.y + frame.height <= layout.height + 0.001);
    for (const other of layout.frames) {
      if (frame === other) continue;
      const separate =
        frame.x + frame.width + FRAME_GAP <= other.x + 0.001 ||
        other.x + other.width + FRAME_GAP <= frame.x + 0.001 ||
        frame.y + frame.height + FRAME_GAP + FRAME_LABEL <= other.y + 0.001 ||
        other.y + other.height + FRAME_GAP + FRAME_LABEL <= frame.y + 0.001;
      assert.ok(separate, `frame and label clearance: ${frame.id}/${other.id}`);
    }
  }
  return layout;
}

test("packing covers empty, single, portrait, landscape and mixed sizes without omissions or overlaps", () => {
  assert.deepEqual(packFrames([]), { frames: [], width: 0, height: 0 });
  for (const aspect of [0.45, 1, 1.5, 2.5])
    for (const count of [1, 3, 4, 9, 100]) {
      assertPacked(dimensions(count, 390, 844), aspect);
      assertPacked(dimensions(count, 1440, 900), aspect);
      assertPacked(
        dimensions(count).map((frame, index) => ({
          ...frame,
          width: [320, 1440, 600, 2800][index % 4],
          height: [1200, 900, 6000, 200][index % 4],
        })),
        aspect,
      );
    }
});

test("three frames favor a row; four and nine landscape frames balance into centered rows", () => {
  for (const frames of [dimensions(3), dimensions(3, 390, 844)])
    assert.equal(
      new Set(packFrames(frames, 1.5).frames.map((frame) => frame.y)).size,
      1,
    );
  assert.equal(
    new Set(packFrames(dimensions(4), 1.5).frames.map((frame) => frame.y)).size,
    2,
  );
  assert.equal(
    new Set(packFrames(dimensions(9), 1.5).frames.map((frame) => frame.y)).size,
    3,
  );
  const rows = packFrames(dimensions(7), 1.5);
  for (const y of new Set(rows.frames.map((frame) => frame.y))) {
    const row = rows.frames.filter((frame) => frame.y === y);
    assert.equal(row[0].x, rows.width - row.at(-1).x - row.at(-1).width);
  }
});

test("board selection translates the pointer, preserves local bounds and respects parent clips", () => {
  const nodes = [
    {
      id: "second",
      parentId: null,
      bounds: { x: 0, y: 0, width: 300, height: 600 },
      clip: true,
    },
    {
      id: "child",
      parentId: "second",
      bounds: { x: 24, y: 32, width: 50, height: 40 },
    },
    {
      id: "overflow",
      parentId: "second",
      bounds: { x: 290, y: 32, width: 50, height: 40 },
    },
  ];
  const frames = [
    { id: "empty", width: 800, height: 600 },
    { id: "second", width: 300, height: 600, scene: { nodes }, revision: 7 },
  ];
  const layout = [
    { x: 0, y: 32 },
    { x: 864, y: 32 },
  ];
  const before = structuredClone(nodes);
  assert.equal(boardHitTest(frames, layout, 864 + 30, 32 + 40, 7)?.id, "child");
  assert.equal(boardHitTest(frames, layout, 864 + 320, 32 + 40, 7), undefined);
  assert.equal(boardHitTest(frames, layout, 830, 40, 7), undefined);
  assert.deepEqual(nodes, before);
  // Only the accepted revision is hit: older and stale scenes are not current.
  assert.equal(boardHitTest(frames, layout, 864 + 30, 32 + 40, 8), undefined);
  const stale = [frames[0], { ...frames[1], stale: true }];
  assert.equal(boardHitTest(stale, layout, 864 + 30, 32 + 40, 7), undefined);
});

test("labels stay 12 CSS pixels at 27% and 5%, with reserved Fit clearance and extreme zoom collision handling", () => {
  for (const scale of [0.27, 0.05]) {
    const metrics = frameLabelMetrics(800, scale, 28 / scale);
    assert.equal(metrics.visible, true);
    assert.equal(metrics.fontSize * scale, 12);
    assert.ok(metrics.y * scale + metrics.height * scale < 0);
  }
  assert.equal(frameLabelMetrics(800, 0.05, 32).visible, false);
  for (const frames of [
    dimensions(9),
    dimensions(9, 8000, 6000),
    [
      { id: "tall", width: 400, height: 9000 },
      { id: "wide", width: 9000, height: 400 },
    ],
  ]) {
    const layout = fitFrameBoard(frames, { width: 800, height: 700 });
    const scale = Math.min(704 / layout.width, 580 / layout.height, 1);
    for (const frame of layout.frames)
      assert.ok(
        frameLabelMetrics(frame.width, scale, frame.labelSpace).visible,
      );
  }
});
