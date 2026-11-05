import { test } from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { authoredCases, buildCorpus, thresholdCases } from "./corpus.mjs";
import { sha256, ROOT } from "./environment.mjs";
import { tableTags, boundedDownload } from "./font-fixtures.mjs";
import { REPORT_SCHEMA, eligibility } from "./schema.mjs";
import { startServer } from "./http-server.mjs";

const font = { manifest: { id: "noto", family: "Noto", hash: "a".repeat(64), weight: 400, style: "normal" } };

test("legacy corpus is read only; all 1232 advance + 120 wrap requests survive", async () => {
  const corpus = await buildCorpus([font]);
  assert.equal(corpus.source.advanceCases, 1232);
  assert.equal(corpus.source.wrapCases, 120);
  assert.equal(new Set(corpus.requests.map((r) => r.key)).size, corpus.requests.length);
  assert.equal(corpus.requests.filter((r) => corpus.cases[r.key].group === "spike-advance").length, 1232);
  const keys = REPORT_SCHEMA.$defs.Request.required.toSorted();
  for (const request of corpus.requests) assert.deepEqual(Object.keys(request).toSorted(), keys);
  const bytes = await readFile(`${ROOT}${corpus.source.path}`);
  assert.equal(corpus.source.sha256, sha256(bytes));
});

test("current corpus explicitly covers whitespace, transforms, tracking and graphemes", () => {
  const cases = authoredCases();
  for (const char of ["\r", "\n", "\t", "\f", "\v", "\u00a0", "\u202f", "\u2060", "\ufeff", "\u0301"])
    assert(cases.some((c) => c.text.includes(char)));
  for (const transform of ["none", "uppercase", "lowercase", "capitalize"])
    assert(cases.some((c) => c.textTransform === transform));
  assert(cases.some((c) => c.letterSpacing < 0));
  assert(cases.some((c) => c.letterSpacing > 0));
  assert(cases.some((c) => c.text.includes("ΟΣ")));
  assert(cases.some((c) => c.text.includes("ß")));
});

test("ceil64 boundary cases preserve full request identity and the exact 1/64 grid", () => {
  const request = { key: "current/noto/0", text: "AV", width: null };
  const cases = { [request.key]: { declaredFallback: false } };
  const result = thresholdCases([request], { [request.key]: { lines: [{}], width: 17.125, missing: [] } }, cases);
  assert.deepEqual(result.map((r) => r.width), [17.109375, 17.125, 17.140625]);
  assert(result.every((r) => r.text === "AV" && cases[r.key].thresholdOf === request.key));
});

test("native .notdef policy is not disguised as browser fallback parity", () => {
  assert.deepEqual(eligibility({ missing: [0x2318], coverageUnavailable: false }, {}), { status: "excluded", reason: "missing-glyph" });
  assert.equal(eligibility({ missing: [], coverageUnavailable: true }, {}).status, "excluded");
  assert.equal(eligibility({ missing: [], coverageUnavailable: false }, { declaredFallback: true }).status, "excluded");
  assert.equal(eligibility({ missing: [], coverageUnavailable: false }, {}).status, "eligible");
});

test("bounded download treats HTTP and size failures as fatal", async () => {
  await assert.rejects(boundedDownload("data:application/octet-stream,too-long", 2), /exceeds/);
  const server = await startServer([]);
  try {
    await assert.rejects(boundedDownload(`${server.url}/does-not-exist`), /Download failed: 404/);
    assert.equal(server.failures.length, 1);
  } finally { await server.close(); }
});

test("SFNT table inventory records actual bytes including variable axes", async () => {
  const bytes = await readFile(`${ROOT}renderer/fonts/NotoSans.ttf`);
  assert(tableTags(bytes).includes("cmap"));
  assert(tableTags(bytes).includes("fvar"));
  assert.throws(() => tableTags(Buffer.alloc(3)), /Truncated/);
});
