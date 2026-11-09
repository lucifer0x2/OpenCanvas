import test from "node:test";
import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { dirname, resolve, join } from "node:path";
import { fileURLToPath } from "node:url";
import { compare } from "./compare.mjs";
import { decodePNG, encodePNG } from "./png.mjs";
import { serve, withOracle } from "./oracle.mjs";
const raster = (width = 200) => ({ width, height: 1, data: Buffer.alloc(width * 4, 255) });
test("delta is >8, and fraction is strictly <0.005", () => {
  const a = raster(), b = raster(); b.data[0] -= 8;
  assert.equal(compare(a, b).report.verdict, "PASS");
  b.data[0]--;
  const result = compare(a, b).report;
  assert.equal(result.visibleStraight.pixelsOver8, 1); assert.equal(result.totalPixels, 200);
  assert.equal(result.visibleStraight.fractionOver8, 0.005); assert.equal(result.verdict, "FAIL");
  const c = raster(201), d = raster(201); d.data[0] -= 9;
  assert.equal(compare(c, d).report.verdict, "PASS");
});
test("max any channel includes alpha and reports coordinates", () => {
  const a = raster(), b = raster(); b.data[7] = 0;
  const m = compare(a, b).report.visibleStraight;
  assert.equal(m.pixelsOver8, 1); assert.deepEqual(m.maxAt, { x: 1, y: 0, deltas: [0, 0, 0, 255] });
});
test("transparent RGB0 differences are reported raw, but not visible gate", () => {
  const a = raster(), b = raster(); a.data[3] = b.data[3] = 0; b.data[0] = 0;
  const m = compare(a, b).report;
  assert.equal(m.rawStraight.pixelsOver8, 1); assert.equal(m.visibleStraight.pixelsOver8, 0);
  b.data[3] = 1;
  assert.equal(compare(a, b).report.visibleStraight.pixelsOver8, 1);
});
test("text cannot turn a mismatch into a pass; no mask or tolerance override", () => {
  const a = raster(), b = raster(); b.data.fill(0);
  const result = compare(a, b, { category: "text" }).report;
  assert.equal(result.verdict, "REVIEW_TEXT"); assert.equal(result.thresholdExceeded, true);
  assert.throws(() => compare(a, b, { mask: [0, 0, 200, 1] }), /not supported/);
  assert.throws(() => compare(a, b, { threshold: 255 }), /not supported/);
});
test("dimensions and empty images fail, never drop pixels", () => {
  assert.throws(() => compare(raster(), raster(199)), /dimensions/);
  assert.throws(() => compare(raster(0), raster(0)), /Empty/);
});
test("PNG roundtrip and corruption detection", () => {
  const a = raster(32); for (let i = 0; i < a.data.length; i++) a.data[i] = i * 7 % 256;
  const png = encodePNG(a); assert.deepEqual(decodePNG(png), a);
  png[20] ^= 1; assert.throws(() => decodePNG(png), /CRC/);
  assert.throws(() => decodePNG(Buffer.from("broken")), /signature/);
});
test("side-by-side order is Chrome, native, diff", () => {
  const a = raster(1), b = raster(1); b.data[0] = 0;
  const result = compare(a, b);
  assert.deepEqual(result.sideBySide.data.subarray(0, 4), a.data);
  assert.deepEqual(result.sideBySide.data.subarray(4, 8), b.data);
  assert.equal(result.sideBySide.width, 3); assert.equal(result.diff.data[0], 255);
});
test("HTTP port zero server closes on success and reports missing routes", async () => {
  const server = await serve(new Map([["/", { type: "text/plain", bytes: Buffer.from("ok") }]]));
  assert.equal(await (await fetch(server.origin)).text(), "ok");
  assert.equal((await fetch(`${server.origin}/missing`)).status, 404);
  await server.close(); await assert.rejects(fetch(server.origin));
});
test("oracle callback errors clean up browser and HTTP server", async () => {
  const root = resolve(dirname(fileURLToPath(import.meta.url)), "../..");
  await mkdir(join(root, "spikes/native-paint-parity/out"), { recursive: true });
  const out = await mkdtemp(join(dirname(fileURLToPath(import.meta.url)), "out/test-"));
  let origin;
  try {
    await assert.rejects(withOracle(root, out, async ({ page }) => {
      origin = new URL(page.url()).origin; throw Error("injected capture failure");
    }), /injected capture failure/);
    await assert.rejects(fetch(origin));
  } finally { await rm(out, { recursive: true, force: true }); }
});
