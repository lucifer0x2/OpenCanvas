// Run: node --test renderer/*.test.mjs (Node strips the types of image-fit.ts).
import assert from "node:assert/strict";
import { readFileSync } from "node:fs";
import { test } from "node:test";
import { IMAGE_FITS, placeImage } from "./image-fit.ts";

const box = { width: 40, height: 40 };
const wide = { width: 200, height: 100 };

test("renderer fit names are the validator's", () => {
  const source = readFileSync(
    new URL("../crates/storage/src/image_fit.rs", import.meta.url),
    "utf8",
  );
  const list = source.match(/IMAGE_FITS: \[&str; \d+\] = \[([^\]]*)\]/)[1];
  assert.deepEqual(
    [...IMAGE_FITS],
    [...list.matchAll(/"([^"]+)"/g)].map((m) => m[1]),
  );
});

test("each fit places the image as documented", () => {
  assert.deepEqual(placeImage("fit", box, wide), {
    x: 0,
    y: 0,
    width: 40,
    height: 40,
  });
  assert.deepEqual(placeImage("contain", box, wide), {
    x: 0,
    y: 10,
    width: 40,
    height: 20,
  });
  assert.deepEqual(placeImage("cover", box, wide), {
    x: -20,
    y: 0,
    width: 80,
    height: 40,
  });
});

test("legacy scene name fill still stretches", () => {
  assert.deepEqual(placeImage("fill", box, wide), placeImage("fit", box, wide));
});
