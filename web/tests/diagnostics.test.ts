import assert from "node:assert/strict";
import test from "node:test";
import { diagnosticLabel } from "../src/diagnostics.ts";

test("viewer labels mixed legacy and structured diagnostics without exposing evidence", () => {
  const labels = [
    "legacy resource warning",
    {
      code: "TEXT_CONTENT_OUTSIDE_BOX",
      nodeId: "instance/label",
      message: "Measured text exceeds its box.",
      examples: [{ privateSource: "do not display" }],
    },
    {
      code: "TEXT_OVERFLOW_SUMMARY",
      sourceSceneKey: "key",
      reportedNodes: 12,
      omittedNodes: 3,
      provenance: { fontManifestHash: "do not display" },
    },
    null,
    { code: 7, message: ["unexpected"] },
  ].map(diagnosticLabel);
  assert.deepEqual(labels, [
    "legacy resource warning",
    "TEXT_CONTENT_OUTSIDE_BOX (instance/label): Measured text exceeds its box.",
    "TEXT_OVERFLOW_SUMMARY: 12 reported, 3 omitted",
    "Design diagnostic",
    "Design diagnostic",
  ]);
  assert.ok(labels.every((label) => !label.includes("[object Object]")));
  assert.ok(labels.every((label) => !label.includes("do not display")));
});
