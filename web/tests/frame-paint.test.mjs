// Publish coalescing and per-frame bitmap cache units (docs/VIEWER-FRAME-PAINT.md).
import assert from "node:assert/strict";
import test from "node:test";
import { sceneModule } from "./scene-module.mjs";

const { publishScheduler, PUBLISH_FALLBACK_MS } = await sceneModule("publishScheduler");
const { bitmapBox, blitOrigin, placement, MAX_BITMAP_PIXELS } =
  await sceneModule("../paint/bitmapBox");
const { BoardPaint, ZOOM_SETTLE_MS } = await sceneModule("../paint/boardPaint");
const { FrameBitmap } = await sceneModule("../paint/frameBitmap");

/** Manual frames and timers. */
function clock() {
  const frames = new Map(), timers = new Map();
  let id = 0;
  return {
    frames, timers,
    requestFrame: (run) => (frames.set(++id, run), id),
    cancelFrame: (key) => frames.delete(key),
    setTimer: (run, ms) => (timers.set(++id, { run, ms }), id),
    clearTimer: (key) => timers.delete(key),
    tick() { const runs = [...frames.values()]; frames.clear(); runs.forEach((run) => run()); },
    expire() { const runs = [...timers.values()]; timers.clear(); runs.forEach((t) => t.run()); },
  };
}

test("publishes coalesce to one commit per animation frame", () => {
  const time = clock();
  let commits = 0;
  const scheduler = publishScheduler(() => commits++, time);
  scheduler.schedule();
  scheduler.schedule();
  scheduler.schedule();
  assert.equal(commits, 0, "nothing commits synchronously");
  time.tick();
  assert.equal(commits, 1);
  assert.equal(time.timers.size, 0, "the fallback timer is cleared by the frame");
  time.expire();
  assert.equal(commits, 1);
  scheduler.schedule();
  time.tick();
  assert.equal(commits, 2, "the next frame can commit again");
});

test("a hidden tab without animation frames still commits through the fallback timer", () => {
  const time = clock();
  let commits = 0;
  const scheduler = publishScheduler(() => commits++, time);
  scheduler.schedule();
  assert.equal([...time.timers.values()][0].ms, PUBLISH_FALLBACK_MS);
  time.expire();
  assert.equal(commits, 1);
  assert.equal(time.frames.size, 0, "the pending frame is cancelled");
  time.tick();
  assert.equal(commits, 1);
  const bare = publishScheduler(() => commits++, { ...time, requestFrame: undefined });
  bare.schedule();
  time.expire();
  assert.equal(commits, 2, "no requestAnimationFrame at all");
});

test("flush commits at once and replaces a scheduled commit; cancel stops everything", () => {
  const time = clock();
  let commits = 0;
  const scheduler = publishScheduler(() => commits++, time);
  scheduler.schedule();
  scheduler.flush();
  assert.equal(commits, 1);
  time.tick();
  time.expire();
  assert.equal(commits, 1, "the scheduled commit was absorbed by the flush");
  scheduler.schedule();
  scheduler.cancel();
  time.tick();
  time.expire();
  scheduler.schedule();
  scheduler.flush();
  time.tick();
  assert.equal(commits, 1, "no publish after abort");
});

const extent = { x0: 0, y0: 0, x1: 100, y1: 50 };
const matrix = (a, e, f) => ({ a, b: 0, c: 0, d: a, e, f });

test("a bitmap box keeps the frame's subpixel phase and blits exactly after a pan", () => {
  const box = bitmapBox(extent, matrix(2, 10.25, 20.5));
  assert.deepEqual(box, { scale: 2, phaseX: 0.25, phaseY: 0.5, x0: -1, y0: -1, width: 203, height: 103 });
  // Device pixel of scene x=0 is 10.25 on the layer and phaseX - x0 = 1.25 in the bitmap.
  assert.deepEqual(blitOrigin(box, matrix(2, 10.25, 20.5)), { x: 9, y: 19 });
  assert.deepEqual(blitOrigin(box, matrix(2, 50.25, -9.5)), { x: 49, y: -11 });
  assert.equal(bitmapBox(extent, { ...matrix(2, 0, 0), b: 0.1 }), undefined, "rotation");
  assert.equal(bitmapBox({ ...extent, x1: 20000 }, matrix(1, 0, 0)), undefined, "side");
  const side = Math.ceil(Math.sqrt(MAX_BITMAP_PIXELS)) + 1;
  assert.equal(bitmapBox({ x0: 0, y0: 0, x1: side, y1: side }, matrix(1, 0, 0)), undefined, "area");
  assert.equal(placement(extent, matrix(1, 0, 0), 300, 200), "visible");
  assert.equal(placement(extent, matrix(1, 350, 0), 300, 200), "near");
  assert.equal(placement(extent, matrix(1, 700, 0), 300, 200), "far");
});

test("a zoom settles after its scale holds; the first scale is at rest", async () => {
  let now = 0, redraws = 0;
  const board = new BoardPaint(() => redraws++, () => now);
  assert.equal(board.settled(1), true);
  now = 10;
  assert.equal(board.settled(1.5), false);
  now = 10 + ZOOM_SETTLE_MS;
  assert.equal(board.settled(1.5), true);
  now = 500;
  assert.equal(board.settled(2), false);
  await new Promise((resolve) => setTimeout(resolve, ZOOM_SETTLE_MS + 30));
  assert.equal(redraws, 1, "the layer is asked to redraw once the zoom rests");
  assert.equal(board.settled(2), true);
  assert.equal(board.reserve(10), true);
  assert.equal(new BoardPaint(() => {}, () => 0, 5).reserve(10), false, "budget");
  board.dispose();
});

/** A fake 2D target: records paints, blits and the transform the bitmap set. */
function target(width = 400, height = 300, m = matrix(1, 0, 0)) {
  const log = [];
  return {
    log,
    canvas: { width, height },
    getTransform: () => m,
    setTransform: (...args) => log.push(["setTransform", ...args]),
    save() {}, restore() {},
    drawImage: (image, x, y) => log.push(["blit", x, y, image.width, image.height]),
  };
}
const content = () => {
  const root = { id: "root" };
  return {
    scene: { id: Math.random() },
    resources: {},
    tree: { roots: [root], extent: new Map([["root", extent]]) },
  };
};

test("a frame bitmap paints once, then blits until its content, zoom or distance changes", () => {
  globalThis.document = {
    createElement: () => ({ width: 0, height: 0, getContext: () => target() }),
  };
  let now = 0;
  const board = new BoardPaint(() => {}, () => now);
  const bitmap = new FrameBitmap(board);
  let paints = 0;
  const paint = () => paints++;
  const frame = content();
  const layer = target();
  bitmap.draw(layer, frame, paint);
  bitmap.draw(layer, frame, paint);
  assert.equal(paints, 1, "one raster, then the bitmap");
  assert.equal(layer.log.filter((entry) => entry[0] === "blit").length, 2);
  assert.equal(board.livePixels, 102 * 52);
  bitmap.draw(layer, { ...frame, scene: { id: "next" } }, paint);
  assert.equal(paints, 2, "a new scene re-rasterizes");
  now = 1000;
  bitmap.draw(target(400, 300, matrix(1.5, 0, 0)), frame, paint);
  assert.equal(paints, 3, "a changing zoom paints vectors");
  assert.equal(board.livePixels, 0, "and drops the old-zoom bitmap");
  now += ZOOM_SETTLE_MS;
  bitmap.draw(target(400, 300, matrix(1.5, 0, 0)), frame, paint);
  bitmap.draw(target(400, 300, matrix(1.5, 40, 0)), frame, paint);
  assert.equal(paints, 4, "rested zoom rasters once; a pan blits");
  bitmap.draw(target(400, 300, matrix(1.5, 500, 0)), frame, paint);
  assert.equal(paints, 4, "off screen paints nothing");
  assert.ok(board.livePixels > 0, "a near frame keeps its bitmap");
  bitmap.draw(target(400, 300, matrix(1.5, 2000, 0)), frame, paint);
  assert.equal(board.livePixels, 0, "a far frame releases it");
  const unbounded = content();
  unbounded.tree.extent.set("root", { x0: -Infinity, y0: 0, x1: 10, y1: 10 });
  bitmap.draw(target(), unbounded, paint);
  assert.equal(paints, 5, "unbounded content paints vectors");
  bitmap.draw(target(), frame, paint);
  bitmap.release();
  assert.equal(board.livePixels, 0, "release frees the pixels");
  board.dispose();
  delete globalThis.document;
});
