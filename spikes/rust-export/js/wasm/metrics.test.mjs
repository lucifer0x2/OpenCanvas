import test from "node:test";
import assert from "node:assert/strict";
import { compare, get, stats, validate } from "./metrics.mjs";
import { runWorker } from "./worker-client.mjs";
import { runCase } from "./case.mjs";

test("nearest rank p50/p95, original samples preserved", () => {
  const samples = Array.from({ length: 20 }, (_, i) => 20 - i);
  const s = stats(30, samples);
  assert.equal(s.p50Ms, 10);
  assert.equal(s.p95Ms, 19);
  assert.deepEqual(s.samplesMs, samples);
  assert.equal(samples[0], 20);
  assert.throws(() => stats(0, []));
  assert.throws(() => stats(0, [NaN]));
});

test("premultiplied RGBA comparison counts pixels, not channels", () => {
  assert.deepEqual(compare([1, 2, 3, 4, 5, 6, 7, 8], [2, 4, 3, 4, 5, 6, 7, 8]), { pixelsDiffer: 1, maxDelta: 2 });
  assert.throws(() => compare([], []));
  assert.throws(() => compare([0, 1, 2, 3], [0]));
});

test("fetch rejects missing or empty native/artifact reads", async () => {
  const previous = globalThis.fetch;
  try {
    globalThis.fetch = async () => new Response("missing", { status: 404 });
    await assert.rejects(get("native.rgba"), /404/);
    globalThis.fetch = async () => new Response("");
    await assert.rejects(get("empty.wasm"), /empty input/);
  } finally { globalThis.fetch = previous; }
});

test("case coverage, dimensions, warm count and parity are mandatory", () => {
  const c = { case: "tile-s2", size: [1440, 900], coldMs: 1, p50Ms: 1, p95Ms: 2, warm: 20, vsNative: { pixelsDiffer: 0, maxDelta: 0 } };
  const cases = [["tile", 2]];
  validate({ cases: [c] }, cases, true);
  assert.throws(() => validate({ cases: [] }, cases));
  assert.throws(() => validate({ cases: [c, c] }, cases));
  assert.throws(() => validate({ cases: [{ ...c, size: [720, 450] }] }, cases));
  assert.throws(() => validate({ cases: [{ ...c, vsNative: null }] }, cases, true));
  assert.throws(() => validate({ error: "trap" }, cases));
});

test("Worker errors, messageerror, timeout, payload errors and post failures terminate", async () => {
  const previous = globalThis.Worker;
  try {
    for (const mode of ["error", "messageerror", "timeout", "payload", "throw", "missing-transfer", "transfer", "ok"]) {
      let terminated = false;
      globalThis.Worker = class {
        postMessage() {
          if (mode === "throw") throw new Error("post failure");
          queueMicrotask(() => {
            if (mode === "error") this.onerror({ message: "crash" });
            if (mode === "messageerror") this.onmessageerror();
            if (mode === "payload") this.onmessage({ data: { error: "trap" } });
            if (mode === "transfer") this.onmessage({ data: { transferProbe: {
              case: "a", buffer: new ArrayBuffer(4), sentAt: performance.timeOrigin + performance.now(),
            } } });
            if (["transfer", "missing-transfer"].includes(mode)) this.onmessage({ data: { cases: [{ case: "a", size: [1, 1] }] } });
            if (mode === "ok") this.onmessage({ data: { cases: [] } });
          });
        }
        terminate() { terminated = true; }
      };
      const promise = runWorker({ timeoutMs: 5 });
      if (mode === "transfer") assert.equal((await promise).cases[0].workerTransferMs >= 0, true);
      else if (mode === "ok") assert.deepEqual(await promise, { cases: [] });
      else await assert.rejects(promise);
      assert.equal(terminated, true, mode);
    }
  } finally { globalThis.Worker = previous; }
});

test("nonzero warm-render return code aborts case", async () => {
  const fetchBefore = globalThis.fetch;
  const instantiateBefore = WebAssembly.instantiate;
  let renders = 0;
  try {
    globalThis.fetch = async () => new Response(new Uint8Array([0, 0, 0, 0]));
    WebAssembly.instantiate = async () => ({ exports: {
      memory: new WebAssembly.Memory({ initial: 1 }), alloc: () => 16, dealloc() {},
      decode_image: () => 0, init_font: () => 0, load_scene: () => 0,
      render: () => ++renders === 1 ? 0 : -2,
    } });
    await assert.rejects(runCase(null, new Uint8Array([1]), new Uint8Array([1]), "spike", 1, 20), /render returned -2/);
    assert.equal(renders, 2);
  } finally { globalThis.fetch = fetchBefore; WebAssembly.instantiate = instantiateBefore; }
});
