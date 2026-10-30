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
    null,
    { code: 7, message: ["unexpected"] },
  ].map(diagnosticLabel);
  assert.deepEqual(labels, [
    "legacy resource warning",
    "TEXT_CONTENT_OUTSIDE_BOX (instance/label): Measured text exceeds its box.",
    "Design diagnostic",
    "Design diagnostic",
  ]);
  assert.ok(labels.every((label) => !label.includes("[object Object]")));
});
