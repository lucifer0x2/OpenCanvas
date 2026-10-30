import { test } from "node:test";
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { parseCoverage, missingCodePoints } from "./glyph-coverage.ts";

const bytes = (file) => { const b = readFileSync(new URL(file, import.meta.url)); return b.buffer.slice(b.byteOffset, b.byteOffset + b.byteLength); };
test("real SFNT cmap covers Vietnamese and rejects absent CJK/scalars", () => {
  for (const file of ["./fonts/NotoSans.ttf", "../fixtures/fonts/Lato-Regular.ttf"]) {
    const coverage = parseCoverage(bytes(file));
    assert.ok(coverage);
    assert.deepEqual(missingCodePoints("Tiếng Việt ă â đ ê ô ơ ư", coverage), []);
    assert.deepEqual(missingCodePoints("Tiếng Việt ă â đ ê ô ơ ư".normalize("NFD"), coverage), []);
    assert.ok(!coverage(0x4e2d)); assert.ok(!coverage(0x10ffff));
  }
});
test("truncated/corrupt SFNT and unsupported container never report fabricated coverage", () => {
  const font = bytes("./fonts/NotoSans.ttf");
  for (const length of [0, 4, 11, 12, 16, 32, 100, 256, 1024]) assert.equal(parseCoverage(font.slice(0, length)), undefined);
  for (const signature of [0x774f4632, 0x774f4646, 0xdeadbeef]) {
    const bad = font.slice(0); new DataView(bad).setUint32(0, signature);
    assert.equal(parseCoverage(bad), undefined);
  }
});
test("coverage accepts raw or canonical graphemes, ignores sequence controls, and deduplicates", () => {
  const raw = cp => [0x61, 0x301].includes(cp);
  assert.deepEqual(missingCodePoints("á", raw), []);
  assert.deepEqual(missingCodePoints("á", cp => cp === 0xe1), []);
  assert.deepEqual(missingCodePoints("a‍a\n a︎ 中中", cp => cp === 0x61), [0x4e2d]);
});
