import test from "node:test";
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { headerFacts, variableCases, WEIGHTS } from "./v1-fixtures.mjs";

test("bounded sweep covers the requested weights, sizes, spacing and semantic strings", () => {
  const fonts = WEIGHTS.map((weight) => ({ manifest: { id: `inter-${weight}`, weight,
    family: "alias", hash: "a".repeat(64), style: "normal" } }));
  const { requests, cases } = variableCases(fonts);
  assert.equal(requests.length, 135);
  assert.equal(new Set(requests.map((r) => r.key)).size, 135);
  assert.deepEqual([...new Set(requests.map((r) => r.fontSize))], [16, 24, 32]);
  assert.deepEqual([...new Set(requests.map((r) => r.letterSpacing))], [0, -2, 0.5]);
  assert.ok(requests.every((r) => r.width === null && !cases[r.key].declaredFallback));
  assert.ok(requests.some((r) => r.text === "AVATAR office Tiếng Việt"));
});

test("font facts read actual fvar/header tables rather than alias metadata", async () => {
  const bytes = await readFile(new URL("../../renderer/fonts/NotoSans.ttf", import.meta.url));
  const facts = headerFacts(bytes);
  assert.deepEqual(facts.axes.find((a) => a.tag === "wght"),
    { tag: "wght", min: 100, default: 400, max: 900 });
  assert.equal(facts.os2.weight, 400);
  assert.equal(facts.head.unitsPerEm, 1000);
  assert.ok(facts.tables.includes("HVAR"));
});
