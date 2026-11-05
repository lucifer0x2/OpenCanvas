import assert from "node:assert/strict";
import test from "node:test";
import { createHash } from "node:crypto";
import { sceneModule } from "./scene-module.mjs";

const { FontByteCache, untilAborted } = await sceneModule("fontBytes");
const bytes = new TextEncoder().encode("immutable font bytes");
const font = {
  id: "font",
  hash: createHash("sha256").update(bytes).digest("hex"),
  url: "/font",
};

test("font bytes are validated once, coalesced and preserved without detachment", async () => {
  const previous = globalThis.fetch;
  let requests = 0;
  globalThis.fetch = async () => {
    requests++;
    return new Response(bytes);
  };
  try {
    const cache = new FontByteCache();
    const [a, b] = await Promise.all([cache.get(font), cache.get(font)]);
    assert.equal(a, b);
    assert.equal(await cache.get(font), a);
    structuredClone(a);
    assert.equal(a.byteLength, bytes.byteLength);
    assert.equal(requests, 1);
    await cache.get({ ...font, id: "other" });
    await cache.get({ ...font, url: "/other" });
    assert.equal(requests, 3);
  } finally {
    globalThis.fetch = previous;
  }
});

test("hash failures are rejected and evicted, while cancellation leaves shared bytes reusable", async () => {
  const previous = globalThis.fetch;
  let requests = 0;
  globalThis.fetch = async () =>
    new Response(++requests === 1 ? "tampered" : bytes);
  try {
    const cache = new FontByteCache();
    await assert.rejects(cache.get(font), /hash mismatch/);
    const valid = await cache.get(font);
    assert.equal(valid.byteLength, bytes.byteLength);
    assert.equal(requests, 2);
    const controller = new AbortController();
    let finish;
    const pending = new Promise((resolve) => {
      finish = resolve;
    });
    const observed = untilAborted(pending, controller.signal);
    controller.abort();
    await assert.rejects(observed, /aborted/);
    finish(valid);
    assert.equal(await cache.get(font), valid);
  } finally {
    globalThis.fetch = previous;
  }
});
