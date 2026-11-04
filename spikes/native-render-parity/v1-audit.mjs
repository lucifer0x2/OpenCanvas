// Compare against retained H2 outputs, never mutate those baselines.
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { OUT } from "./environment.mjs";

const read = async (name) => JSON.parse(await readFile(new URL(`${name}.json`, OUT)));
const baselines = {};
for (const [label, before] of [["primary", "h2-primary-native"],
  ["supplemental", "h2-probe-native"], ["h1", "h2-native"], ["h2", "h2-targeted-native"]]) {
  const old = await read(before), current = await read(`v1-${label}-native`);
  let count = 0;
  for (const key of Object.keys(old.measurements)) {
    assert.deepEqual(current.measurements[key], old.measurements[key], `${label}/${key} measurement`);
    assert.deepEqual(current.probes[key], old.probes[key], `${label}/${key} raw probe`);
    count++;
  }
  assert.deepEqual(current.errors, old.errors);
  assert.equal(Object.keys(current.measurements).length, count);
  baselines[label] = { requests: count, changedMeasurements: 0, changedProbes: 0 };
}

const capture = await read("v1-oracle");
let controls = 0;
for (const request of capture.requests.filter((r) => r.width === null)) {
  const regularId = request.fontId === "v1-static-alias" ? "v1-static-regular" :
    request.fontId.endsWith("-italic") && request.fontId.startsWith("v1-inter-") ?
      request.fontId.replace(/-italic$/, "") : null;
  if (!regularId) continue;
  const other = capture.requests.find((r) => r.fontId === regularId &&
    r.fontSize === request.fontSize && r.letterSpacing === request.letterSpacing && r.text === request.text);
  assert.ok(other);
  assert.deepEqual(capture.browser.measurements[request.key], capture.browser.measurements[other.key]);
  for (const field of ["empty", "fullText", "lines"]) assert.deepEqual(
    capture.browser.probes[request.key][field], capture.browser.probes[other.key][field]);
  controls++;
}
assert.equal(controls, 81);
const native = await read("v1-native");
const rawAdvance = { total: 0, exact: 0, nonIdentical: 0, maxError: 0 };
for (const request of capture.requests.filter((r) => r.width === null)) {
  const error = Math.abs(capture.browser.probes[request.key].fullText.width -
    native.probes[request.key].advance);
  rawAdvance.total++;
  if (error === 0) rawAdvance.exact++; else rawAdvance.nonIdentical++;
  rawAdvance.maxError = Math.max(rawAdvance.maxError, error);
}
const report = { baselines, rawAdvance, browserDescriptorControls: { exact: controls,
  explanation: "Inter lacks ital/slnt; aliased static Lato Regular remains regular bytes without faux styles" } };
await writeFile(new URL("v1-audit.json", OUT), JSON.stringify(report, null, 2));
console.log(JSON.stringify(report, null, 2));
