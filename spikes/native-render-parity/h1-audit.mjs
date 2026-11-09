// Compare raw G2 probes separately from geometry/ink. Requires retained G2 files.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { OUT } from "./environment.mjs";
const read = async (name) => JSON.parse(await readFile(new URL(name, OUT)));
for (const [baseline, current] of [
  ["native-measurements.json", "h1-primary-native.json"],
  ["probe-native.json", "h1-probe-native.json"],
]) {
  const before = await read(baseline), after = await read(current);
  assert.equal(after.oracleSha256, before.oracleSha256);
  // Includes transformed source, full/line raw advances, and optical metrics.
  assert.deepEqual(after.probes, before.probes);
  console.log(JSON.stringify({ baseline, current, unchangedProbes: Object.keys(after.probes).length }));
}
const oracle = await read("oracle.json"), native = await read("h1-primary-native.json");
const expected = oracle.browser.measurements["current/inter/86"].lines[5];
const actual = native.measurements["current/inter/86"].lines[5];
assert.equal(expected.text, "ς i̇");
assert.equal(actual.text, expected.text);
assert.equal(expected.inkBounds.width, 15.574615478515625);
assert.equal(actual.inkBounds.width, 15.574670791625977);
assert.notDeepEqual(actual.inkBounds, expected.inkBounds);
console.log(JSON.stringify({ key: "current/inter/86", fixedAttachment: true,
  remainingStrictInkWidthError: actual.inkBounds.width - expected.inkBounds.width }));
