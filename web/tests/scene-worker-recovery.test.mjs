import assert from "node:assert/strict";
import test from "node:test";
import { Worker } from "node:worker_threads";
import {
  SceneWorkerClient, sceneWorkerRuntime, WorkerStub, computer,
  deferred, harness, message, signal,
} from "./scene-worker-support.mjs";

test("runtime retries a failed initialization promise on its next request", async () => {
  let attempts = 0;
  const replies = [];
  const handle = sceneWorkerRuntime(async () => {
    if (++attempts === 1) throw new Error("module failed");
    return computer();
  }, (reply) => { if (!("started" in reply)) replies.push(reply); });
  await handle({ ...message(), id: 1, kind: "prepare" });
  await handle({ ...message(2), id: 2, kind: "prepare" });
  assert.equal(replies[0].error, "module failed");
  assert.deepEqual(replies[1].result, []);
  assert.equal(attempts, 2);
});

test("aborted prepare r1 followed by failed initialization resets worker and prepare r2 succeeds", async () => {
  const initialization = deferred();
  let attempts = 0;
  const { client, workers } = harness(() => ++attempts === 1
    ? initialization.promise : Promise.resolve(computer()));
  const abort = new AbortController();
  const first = client.prepare(message(), abort.signal);
  await Promise.resolve();
  abort.abort();
  await assert.rejects(first, /cancelled/);
  const waiting = client.prepare(message(2), signal());
  initialization.reject(new Error("module failed"));
  assert.deepEqual(await waiting, []);
  assert.equal(workers[0].terminated, true);
  assert.equal(workers.length, 2);
  assert.ok(workers[1].messages[0].snapshot);
});

test("unknown-id infrastructure replies reset and replay live pending callers", async () => {
  const workers = [];
  const client = new SceneWorkerClient(() => {
    const worker = new WorkerStub();
    workers.push(worker);
    return worker;
  });
  const pending = client.compute(message(), signal());
  workers[0].reply({ id: 999, error: "broken module" });
  assert.equal(workers[0].terminated, true);
  assert.ok(workers[1].messages[0].snapshot);
  workers[1].reply({ id: 1, result: { replayed: true } });
  assert.deepEqual(await pending, { replayed: true });
});

test("replayed queued work keeps its original abort listener and sends full font bytes", async () => {
  const workers = [];
  const client = new SceneWorkerClient(() => {
    const worker = new WorkerStub();
    workers.push(worker);
    return worker;
  });
  const abort = new AbortController();
  const pending = client.compute(message(), abort.signal);
  workers[0].reply({ id: 999, error: "broken module" });
  assert.equal(workers[1].messages[0].fonts.length, 1);
  workers[1].reply({ id: 1, started: "computing" });
  abort.abort();
  await assert.rejects(pending, /cancelled/);
  workers[1].reply({ id: 1, result: {} });
  assert.equal(workers[1].terminated, false);
});

test("structured Rust errors retain code without discarding the warm worker", async () => {
  let invalid = true, loads = 0;
  const { client, workers } = harness(async () => computer({
    load_font() { loads++; },
    compute_scene() {
      if (invalid) throw { code: "INVALID_REQUEST", message: "invalid viewport" };
      return "{}";
    },
  }));
  await assert.rejects(client.compute(message(), signal()), {
    code: "INVALID_REQUEST", message: "invalid viewport",
  });
  invalid = false;
  await client.compute(message(), signal());
  assert.equal(workers.length, 1);
  assert.equal(workers[0].terminated, false);
  assert.equal(workers[0].messages[1].fonts.length, 1, "failed requests do not ack fonts");
  assert.equal(loads, 1, "runtime safely deduplicates successful partial registration");
});

test("queued current work waits beyond timeout behind obsolete compute then succeeds", async () => {
  const worker = new WorkerStub();
  const client = new SceneWorkerClient(() => worker, 20);
  const abort = new AbortController();
  const obsolete = client.compute(message(), abort.signal);
  worker.reply({ id: 1, started: "computing" });
  abort.abort();
  await assert.rejects(obsolete, /cancelled/);
  const current = client.compute(message(2), signal());
  await new Promise((resolve) => setTimeout(resolve, 70));
  assert.equal(worker.terminated, false);
  worker.reply({ id: 1, result: {} });
  worker.reply({ id: 2, started: "initializing" });
  worker.reply({ id: 2, started: "computing" });
  worker.reply({ id: 2, result: { current: true } });
  assert.deepEqual(await current, { current: true });
});

test("active computation and initialization remain bounded, including aborted initialization", async () => {
  for (const phase of ["computing", "initializing"]) {
    const worker = new WorkerStub();
    const client = new SceneWorkerClient(() => worker, 15);
    const pending = client.compute(message(), signal());
    worker.reply({ id: 1, started: phase });
    await assert.rejects(pending, /timed out/);
    assert.equal(worker.terminated, true);
  }
  const { client, workers } = harness(() => new Promise(() => {}), 15);
  const abort = new AbortController();
  const obsolete = client.prepare(message(), abort.signal);
  abort.abort();
  await assert.rejects(obsolete, /cancelled/);
  await assert.rejects(client.prepare(message(2), signal()), /timed out/);
  assert.equal(workers[0].terminated, true);
});

test("initialization and computation share one execution timeout budget", async (t) => {
  t.mock.timers.enable({ apis: ["setTimeout"] });
  const worker = new WorkerStub();
  const client = new SceneWorkerClient(() => worker, 100);
  const pending = client.compute(message(), signal());
  const rejected = assert.rejects(pending, /timed out/);
  worker.reply({ id: 1, started: "initializing" });
  t.mock.timers.tick(70);
  worker.reply({ id: 1, started: "computing" });
  t.mock.timers.tick(31);
  await rejected;
  assert.equal(worker.terminated, true);
});

test("real serial Worker lets cheap current work survive obsolete compute longer than timeout", async (t) => {
  const started = deferred();
  let terminated = false;
  const client = new SceneWorkerClient(() => {
    const thread = new Worker(new URL("./scene-worker-thread.mjs", import.meta.url));
    const adapter = {
      postMessage(message) { thread.postMessage(message); },
      terminate() { terminated = true; void thread.terminate(); },
    };
    thread.on("message", (data) => {
      adapter.onmessage({ data });
      if (data.id === 2 && data.started === "computing") started.resolve();
    });
    thread.on("error", () => adapter.onerror({ preventDefault() {} }));
    return adapter;
  }, 2000);
  t.after(() => client.clearSnapshotCache());
  await client.compute(message(), signal());
  client.timeoutMs = 30;
  const abort = new AbortController();
  const slow = message(2);
  slow.snapshot.delay = 150;
  const obsolete = client.compute(slow, abort.signal);
  await started.promise;
  abort.abort();
  await assert.rejects(obsolete, /cancelled/);
  const before = performance.now();
  await client.compute(message(3), signal());
  assert.ok(performance.now() - before > 60, "queue wait exceeded processing timeout");
  assert.equal(terminated, false);
});
