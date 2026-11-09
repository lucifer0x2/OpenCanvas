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

test("count and byte limits evict least recently used entries independently", async () => {
  const calls = [];
  const fetchBytes = async (item) => {
    calls.push(item.id);
    return new ArrayBuffer(4);
  };
  const a = { ...font, id: "a" };
  const b = { ...font, id: "b" };
  const c = { ...font, id: "c" };
  for (const [count, budget] of [
    [2, 100],
    [10, 8],
  ]) {
    calls.length = 0;
    const cache = new FontByteCache(fetchBytes, count, budget);
    const first = await cache.get(a);
    await cache.get(b);
    assert.equal(await cache.get(a), first); // a is now newer than b.
    await cache.get(c);
    assert.equal(await cache.get(a), first);
    await cache.get(b);
    assert.deepEqual(calls, ["a", "b", "c", "b"]);
  }
});

test("oversized font serves its callers without displacing retained fonts", async () => {
  const calls = [];
  const cache = new FontByteCache(
    async (item) => {
      calls.push(item.id);
      return new ArrayBuffer(item.id === "large" ? 9 : 4);
    },
    10,
    8,
  );
  const small = await cache.get(font);
  const large = { ...font, id: "large" };
  const pending = cache.get(large);
  assert.equal(cache.get(large), pending);
  assert.equal((await pending).byteLength, 9);
  assert.equal(await cache.get(font), small);
  await cache.get(large);
  assert.deepEqual(calls, ["font", "large", "large"]);
});

function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => {
    resolve = yes;
    reject = no;
  });
  return { promise, resolve, reject };
}

test("evicted pending success and failure cannot affect their replacement", async () => {
  for (const fail of [false, true]) {
    const loads = [];
    const cache = new FontByteCache(
      () => {
        const load = deferred();
        loads.push(load);
        return load.promise;
      },
      1,
      8,
    );
    const old = cache.get(font);
    const observed = old.catch((error) => error);
    const other = cache.get({ ...font, id: "other" });
    const replacement = cache.get(font);
    const retained = new ArrayBuffer(8);
    loads[2].resolve(retained);
    assert.equal(await replacement, retained);
    if (fail) loads[0].reject(new Error("old request failed"));
    else loads[0].resolve(new ArrayBuffer(8));
    await observed;
    loads[1].resolve(new ArrayBuffer(8));
    await other;
    assert.equal(cache.get(font), replacement);
    assert.equal(await cache.get(font), retained);
    assert.equal(loads.length, 3);
  }
});

test("concurrent completions enforce byte budget and release evicted accounting", async () => {
  const loads = [];
  const cache = new FontByteCache(
    () => {
      const load = deferred();
      loads.push(load);
      return load.promise;
    },
    10,
    8,
  );
  const a = cache.get(font);
  const other = { ...font, id: "other" };
  const b = cache.get(other);
  loads[0].resolve(new ArrayBuffer(8));
  await a;
  loads[1].resolve(new ArrayBuffer(8));
  const retained = await b;
  const replacement = cache.get(font);
  loads[2].resolve(new ArrayBuffer(4));
  await replacement; // b was evicted; retained bytes must now be four.
  const c = cache.get({ ...font, id: "third" });
  loads[3].resolve(new ArrayBuffer(4));
  await c;
  assert.equal(cache.get(font), replacement);
  assert.equal(retained.byteLength, 8); // Eviction never detaches consumers' bytes.
  assert.equal(loads.length, 4);
});
