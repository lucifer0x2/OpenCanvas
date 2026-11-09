import test from "node:test";
import assert from "node:assert/strict";
import { compare, differences } from "./compare.mjs";

const measurement = () => ({ width: 16, height: 24, ascent: 17,
  lines: [{ text: "A", x: 0, y: 18, width: 16,
    inkBounds: { x: 0, y: -12, width: 15, height: 12 } }],
  missing: [], coverageUnavailable: false });
function fixture() {
  const oracle = { requests: [{ key: "a", fontId: "test", text: "A" }],
    cases: { a: { group: "test", label: "covered", declaredFallback: true } },
    browser: { measurements: { a: measurement() }, eligibility: {
      a: { status: "excluded", reason: "declared-platform-fallback" } } } };
  const native = { measurements: { a: measurement() }, errors: {} };
  return { oracle, native };
}
test("covered declared fallback is still a candidate; ink is separate", () => {
  const { oracle, native } = fixture();
  native.measurements.a.lines[0].inkBounds.width -= 0.1;
  const result = compare(oracle, native);
  assert.equal(result.total.candidates, 1);
  assert.equal(result.total.exactLayout, 1);
  assert.equal(result.total.mismatchInk, 1);
  assert.equal(result.sceneBoundsTested, false);
});
test("no rounding hides a 1/64 fit mismatch or IEEE f32 wire error", () => {
  const { oracle, native } = fixture();
  native.measurements.a.width -= 1 / 64;
  const result = compare(oracle, native);
  assert.equal(result.total.mismatchLayout, 1);
  assert.equal(result.total.maxLayoutError, 1 / 64);
  const [wire] = differences(0.1, Math.fround(0.1));
  assert.equal(wire.f32WireOnly, true);
  assert.notEqual(wire.error, 0);
});
test("actual missing glyphs retain original metadata and exclusion evidence", () => {
  const { oracle, native } = fixture();
  oracle.browser.measurements.a.missing = [0x2318];
  native.measurements.a.missing = [0x2318];
  const result = compare(oracle, native);
  assert.equal(result.total.excluded, 1);
  assert.equal(result.total.candidates, 0);
  assert.deepEqual(result.excluded[0].nativeMissing, [0x2318]);
});
