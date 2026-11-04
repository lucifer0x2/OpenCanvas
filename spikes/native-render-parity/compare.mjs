import { readFile, writeFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { pathToFileURL } from "node:url";

// No rounding or tolerance: a 1/64 discrepancy is always retained.
export function differences(expected, actual, path = "") {
  if (expected === actual) return [];
  if (typeof expected === "number" && typeof actual === "number") {
    return [{ path, expected, actual, error: actual - expected,
      f32WireOnly: Math.fround(expected) === actual }];
  }
  if (expected !== null && actual !== null && typeof expected === "object"
      && typeof actual === "object") {
    const keys = [...new Set([...Object.keys(expected), ...Object.keys(actual)])];
    return keys.flatMap((key) => differences(expected[key], actual[key], `${path}/${key}`));
  }
  return [{ path, expected: expected ?? null, actual: actual ?? null }];
}

function layout(measurement) {
  if (!measurement) return null;
  return { width: measurement.width, height: measurement.height, ascent: measurement.ascent,
    // Bounds derivable from a measurement: origin (0,0), width and height. Full
    // scene/Taffy bounds are NOT captured here and are a separate integration gate.
    lines: measurement.lines.map(({ text, x, y, width }) => ({ text, x, y, width })) };
}

export function compare(oracle, native) {
  const rows = [], tables = {}, total = { requests: 0, candidates: 0, excluded: 0,
    exactLayout: 0, mismatchLayout: 0, exactInk: 0, mismatchInk: 0,
    errors: 0, metadataMismatch: 0, maxLayoutError: 0, maxInkError: 0 };
  const excluded = [];
  for (const request of oracle.requests) {
    const { key, fontId } = request, testCase = oracle.cases[key];
    const expected = oracle.browser.measurements[key], actual = native.measurements[key];
    const missing = [...new Set([...(expected.missing ?? []), ...(actual?.missing ?? [])])];
    // The ONLY authorized exception is actual missing glyphs. An oracle's declared
    // platform fallback label cannot exclude cmap-covered glyphs in native.
    const exclusion = missing.length ? "missing-glyph" : null;
    const geometry = differences(layout(expected), layout(actual));
    const ink = differences(expected.lines.map((l) => l.inkBounds),
      actual?.lines.map((l) => l.inkBounds));
    const metadata = differences({ missing: expected.missing,
      coverageUnavailable: expected.coverageUnavailable },
    actual && { missing: actual.missing, coverageUnavailable: actual.coverageUnavailable });
    const row = { key, fontId, group: testCase.group, label: testCase.label,
      oracleEligibility: oracle.browser.eligibility[key], exclusion, missing,
      geometry, ink, metadata, error: native.errors[key] ?? null };
    rows.push(row);
    if (exclusion) excluded.push({ key, fontId, group: testCase.group, text: request.text,
      browserMissing: expected.missing, nativeMissing: actual?.missing ?? null,
      oracleEligibility: row.oracleEligibility });
    for (const bucket of [total, tables[fontId] ??= {},
      tables[`${fontId}/${testCase.group}`] ??= {}]) {
      bucket.requests = (bucket.requests ?? 0) + 1;
      bucket[exclusion ? "excluded" : "candidates"] =
        (bucket[exclusion ? "excluded" : "candidates"] ?? 0) + 1;
      if (row.error) bucket.errors = (bucket.errors ?? 0) + 1;
      if (metadata.length) bucket.metadataMismatch = (bucket.metadataMismatch ?? 0) + 1;
      if (exclusion) continue;
      for (const [name, diffs] of [["Layout", geometry], ["Ink", ink]]) {
        const field = `${diffs.length ? "mismatch" : "exact"}${name}`;
        bucket[field] = (bucket[field] ?? 0) + 1;
        bucket[`max${name}Error`] = Math.max(bucket[`max${name}Error`] ?? 0,
          ...diffs.map((d) => Math.abs(d.error ?? 0)));
      }
    }
  }
  return { schemaVersion: 1, kind: "native-chromium-text-comparison",
    policy: "strict numeric equality; only actual missing glyphs excluded; ink separate",
    sceneBoundsTested: false, gates: {
      measurementGeometry: total.mismatchLayout === 0 && total.errors === 0 && total.metadataMismatch === 0,
      inkExact: total.mismatchInk === 0 && total.errors === 0,
      fullSceneBounds: "not-tested", rasterPixels: "not-tested" }, total, tables, excluded, rows };
}

async function main() {
  const base = new URL("./out/", import.meta.url);
  const oraclePath = process.argv[2] ?? new URL("oracle.json", base);
  const nativePath = process.argv[3] ?? new URL("native-measurements.json", base);
  const output = process.argv[4] ?? new URL("comparison.json", base);
  const bytes = await readFile(oraclePath), oracle = JSON.parse(bytes);
  const native = JSON.parse(await readFile(nativePath));
  if (native.oracleSha256 !== createHash("sha256").update(bytes).digest("hex")) {
    throw new Error("native measurements belong to a different oracle capture");
  }
  const result = compare(oracle, native);
  await writeFile(output, JSON.stringify(result, null, 2));
  console.log(JSON.stringify({ total: result.total,
    fonts: Object.fromEntries(Object.entries(result.tables).filter(([k]) => !k.includes("/"))) }, null, 2));
  // Deliberately nonzero on layout failure: producing a report is not AC success.
  if (!result.gates.measurementGeometry) process.exitCode = 1;
}
if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) await main();
