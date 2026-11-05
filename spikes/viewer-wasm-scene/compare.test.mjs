import assert from "node:assert/strict";
import test from "node:test";
import { compare, differences } from "./compare.mjs";

test("exact recursive gate rejects numeric epsilon, node order, and contract additions", () => {
  assert.equal(differences({ x: 1 }, { x: 1 + Number.EPSILON }).length, 1);
  assert.equal(differences(["a", "b"], ["b", "a"]).length, 2);
  assert.equal(differences({ x: 1 }, { x: 1, y: 2 }).length, 1);
  assert.equal(differences({ x: null }, {}).length, 1);
});

test("server provenance exclusions preserve advisories, baselines, diagnostics and unknown fields", () => {
  const native = {
    provenance: { engine: "native", sourceSceneKey: "pipeline-key" },
    nodes: [
      { id: "text", kind: "text", lines: [{ text: "Missing 🦄", y: 20 }] },
    ],
    diagnostics: [
      "MISSING_GLYPH node=text U+1F984",
      {
        code: "TEXT_CONTENT_OUTSIDE_BOX",
        sourceSceneKey: "pipeline-key",
        outsideLineCount: 1,
      },
    ],
  };
  const wasm = structuredClone(native);
  delete wasm.provenance.engine;
  wasm.provenance.sourceSceneKey = "core-key";
  wasm.diagnostics[1].sourceSceneKey = "core-key";
  assert.equal(compare(native, wasm).mismatches.length, 0);
  wasm.diagnostics[1].outsideLineCount = 2;
  assert.equal(compare(native, wasm).mismatches.length, 1);
  wasm.nodes[0].lines[0].y += 0.000001;
  wasm.newContractField = true;
  assert.equal(compare(native, wasm).mismatches.length, 3);
  wasm.diagnostics.shift();
  assert.ok(compare(native, wasm).mismatches.length > 3);
});
