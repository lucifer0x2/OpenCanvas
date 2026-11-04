import test from "node:test";
import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compare } from "../native-paint-parity/compare.mjs";
import { serve, withOracle } from "../native-paint-parity/oracle.mjs";
import { expected, checkReference, effect } from "./reference.mjs";
import { changed, browserIdentity } from "./runtime.mjs";
const here = dirname(fileURLToPath(import.meta.url));
const raster = (width, pixel) => ({ width, height: 1, data: Buffer.from(Array.from({ length: width }, () => pixel).flat()) });

test("unchanged gate is >8 and strictly <0.005 over all pixels", () => {
  const a = raster(200, [128, 64, 32, 255]), b = raster(200, [128, 64, 32, 255]);
  b.data[0] += 8; assert.equal(compare(a, b).report.verdict, "PASS");
  b.data[0]++; const result = compare(a, b).report;
  assert.equal(result.totalPixels, 200); assert.equal(result.visibleStraight.fractionOver8, 0.005);
  assert.equal(result.verdict, "FAIL");
  const c = raster(201, [128, 64, 32, 255]), d = raster(201, [128, 64, 32, 255]);
  d.data[0] += 9; assert.equal(compare(c, d).report.verdict, "PASS");
});
test("only both-zero alpha ignores RGB; alpha and low-alpha RGB still gate", () => {
  const a = raster(1, [200, 0, 0, 0]), b = raster(1, [0, 200, 0, 0]);
  assert.equal(compare(a, b).report.visibleStraight.pixelsOver8, 0);
  b.data[3] = 1; assert.equal(compare(a, b).report.verdict, "FAIL");
  a.data[3] = 1; assert.equal(compare(a, b).report.verdict, "FAIL");
  b.data.set(a.data); b.data[3] += 9; assert.equal(compare(a, b).report.verdict, "FAIL");
  assert.throws(() => compare(a, b, { mask: [] }), /not supported/);
});
test("P3 reference differs in-gamut from sRGB, and gray reference is linear", () => {
  assert.deepEqual(expected({ profile: "display-p3", tagged: true }, 0), [193, 95, 98, 255]);
  assert.deepEqual(expected({ profile: "gray-linear", tagged: true, alpha: true }, 16), [188, 188, 188, 192]);
  assert.deepEqual(expected({ profile: "srgb", tagged: true }, 0), [180, 100, 100, 255]);
  const cmyk = expected({ profile: "cmyk-constant-xyz", tagged: true }, 0);
  assert.equal(cmyk[3], 255); assert.ok(cmyk.slice(0, 3).every(v => v > 150));
});
test("reference includes exact alpha and tolerates only bounded surface quantization", () => {
  const f = { profile: "display-p3", tagged: true, alpha: true }, a = raster(1, expected(f, 0));
  assert.equal(checkReference(f, a).pass, true);
  a.data[0] += 4; assert.equal(checkReference(f, a).pass, true);
  a.data[0]++; assert.equal(checkReference(f, a).pass, false);
  a.data.set(expected(f, 0)); a.data[3]--; assert.equal(checkReference(f, a).pass, false);
});
test("nontrivial profiles must have observed effect; sRGB control must not", () => {
  const a = raster(1, [180, 100, 100, 255]), b = raster(1, [193, 94, 97, 255]);
  assert.equal(effect("display-p3", a, a).pass, false);
  assert.equal(effect("display-p3", b, a).pass, true);
  assert.equal(effect("srgb", a, a).pass, true); assert.equal(effect("srgb", b, a).pass, false);
});
test("source guard detects mutations, additions and removal", () => {
  assert.deepEqual(changed({ a: "1", b: "2" }, { a: "3", c: "4" }).sort(), ["a", "b", "c"]);
  assert.deepEqual(changed({ a: "1" }, { a: "1" }), []);
});
test("loopback fixture server has ephemeral port and cleanup", async () => {
  const server = await serve(new Map([["/resource", { type: "application/octet-stream", bytes: Buffer.from([1, 2, 3]) }]]));
  try { assert.deepEqual(Buffer.from(await (await fetch(`${server.origin}/resource`)).arrayBuffer()), Buffer.from([1, 2, 3])); }
  finally { await server.close(); }
  await assert.rejects(fetch(server.origin));
});
test("borrowed Chromium oracle closes temporary resources on failed capture", async () => {
  await mkdir(join(here, "out"), { recursive: true });
  const out = await mkdtemp(join(here, "out/test-")); let origin;
  try {
    await assert.rejects(withOracle(resolve(here, "../.."), out, async ({ page }) => {
      origin = new URL(page.url()).origin;
      const identity = await browserIdentity(page.context().browser());
      assert.match(identity.runtime.product, /156\.0\.8078\.4/);
      assert.match(identity.executableHash, /^[a-f0-9]{64}$/);
      assert.ok(identity.launchCommandLine.includes("--force-color-profile=srgb"));
      throw Error("injected capture failure");
    }), /injected capture failure/);
    await assert.rejects(fetch(origin));
  } finally { await rm(out, { force: true, recursive: true }); }
});
