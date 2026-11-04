import test from "node:test";
import assert from "node:assert/strict";
import { fixtures } from "./fixtures.mjs";
import { probes } from "./probes.mjs";
import { reportSchema } from "./report-schema.mjs";
const resources = {
  fonts: ["noto", "lato-regular", "lato-bold", "lato-italic", "inter"].map(id => ({ id })),
  assets: ["png", "jpeg", "jpeg-orientation6", "webp"].map(id => ({ id })),
};
test("all cases are explicit scene-3/web-4; isolated nontext gates have no text", () => {
  const cases = fixtures(resources), names = new Set();
  for (const c of cases) {
    assert(!names.has(c.name)); names.add(c.name);
    assert(c.scales.includes(1) && c.scales.includes(2));
    assert.equal(c.scene.provenance.sceneVersion, "scene-3");
    assert.equal(c.scene.provenance.rendererVersion, "web-4");
    assert(c.scene.provenance.viewport.width > 0);
    if (c.category === "nontext") assert(c.scene.nodes.every(n => n.kind !== "text"));
    for (const n of c.scene.nodes) if (n.parentId) assert(c.scene.nodes.some(p => p.id === n.parentId));
    const ids = c.scene.nodes.map(n => n.id); assert.equal(new Set(ids).size, ids.length);
  }
  assert(names.has("full-nontext-atlas") && names.has("full-text-atlas"));
});
test("G2 lowercase/23 preserves exact saved Greek/Latin combining-dot line", () => {
  const c = fixtures(resources).find(c => c.name === "text-inter-lowercase-expansion");
  const n = c.scene.nodes[0];
  assert.equal(n.fontSize, 16); assert.equal(n.lineHeight, 23);
  assert.deepEqual(n.lines[5], { text: "ς i\u0307", x: 0, y: 132, width: 17.296875 });
  assert(c.scales.includes(3)); assert.equal(n.lines.length, 7);
});
test("font faces are explicit IDs, crop/fit and every stroke alignment are isolated", () => {
  const cases = fixtures(resources), find = name => cases.find(c => c.name === name);
  assert.deepEqual(find("text-static-faces").scene.fonts.map(f => f.id), ["lato-regular", "lato-bold", "lato-italic"]);
  for (const align of ["inside", "center", "outside"]) assert.equal(find(`stroke-${align}-sides`).scene.nodes[0].stroke.align, align);
  for (const id of resources.assets.map(a => a.id)) for (const fit of ["fit", "contain", "cover"]) assert(find(`image-${id}-${fit}`));
  assert(find("image-normalized-crop").scene.nodes[0].crop);
  assert.deepEqual(find("scale-fractional-ceil").scales, [1, 2, 2.4]);
});
test("error probes cover corruption, missing bytes, hash, versions and budgets", () => {
  assert.deepEqual(probes(resources).map(p => p.name), ["pipeline-version", "scale-zero", "target-budget", "missing-image", "hash-mismatch-image", "decode-corrupt-image", "decode-corrupt-font"]);
  assert.equal(reportSchema.properties.schema.const, "native-paint-parity/1");
});
