import assert from "node:assert/strict";
import test from "node:test";
import { sceneModule } from "./scene-module.mjs";

const { SceneWorkerClient } = await sceneModule("workerClient");
class WorkerStub {
  messages = [];
  terminated = false;
  postMessage(message, transfers) {
    assert.equal(transfers, undefined);
    this.messages.push(structuredClone(message));
  }
  terminate() {
    this.terminated = true;
  }
  reply(id, result) {
    this.onmessage({ data: { id, result } });
  }
}
const message = {
  request: { projectId: "p", revision: 1 },
  snapshot: { projectId: "p", revision: 1 },
  fonts: [{ id: "font", hash: "hash", bytes: new Uint8Array([1, 2]).buffer }],
  urls: {},
};

test("worker stays lazy, correlates replies and never transfers cached font buffers", async () => {
  const workers = [];
  const client = new SceneWorkerClient(() => {
    const worker = new WorkerStub();
    workers.push(worker);
    return worker;
  });
  assert.equal(workers.length, 0);
  const controller = new AbortController();
  const pending = client.compute(message, controller.signal);
  assert.equal(message.fonts[0].bytes.byteLength, 2);
  workers[0].reply(999, { obsolete: true });
  workers[0].reply(workers[0].messages[0].id, { current: true });
  assert.deepEqual(await pending, { current: true });
  controller.abort();
  assert.equal(
    workers[0].terminated,
    false,
    "completed request removed abort handler",
  );
});

test("abort drops one caller and late replies while concurrent and later work reuse the worker", async () => {
  const workers = [];
  const client = new SceneWorkerClient(() => {
    const worker = new WorkerStub();
    workers.push(worker);
    return worker;
  });
  const old = new AbortController();
  const first = client.compute(message, old.signal);
  const concurrent = client.compute(message, new AbortController().signal);
  old.abort();
  await assert.rejects(first, /cancelled/);
  assert.equal(workers[0].terminated, false);
  workers[0].reply(1, { old: true });
  workers[0].onmessage({ data: { id: 1, error: "obsolete error", errorCode: "INVALID_REQUEST" } });
  workers[0].reply(2, { concurrent: true });
  assert.deepEqual(await concurrent, { concurrent: true });
  const next = client.compute(message, new AbortController().signal);
  assert.equal(workers.length, 1);
  assert.equal(workers[0].messages[1].snapshot, undefined);
  assert.equal(workers[0].messages[2].snapshot, undefined);
  workers[0].reply(3, { next: true });
  assert.deepEqual(await next, { next: true });
});

test("real worker errors after abort still reset the worker and re-prime its snapshot", async () => {
  for (const event of ["onerror", "onmessageerror"]) {
    const workers = [];
    const client = new SceneWorkerClient(() => {
      const worker = new WorkerStub();
      workers.push(worker);
      return worker;
    });
    const controller = new AbortController();
    const aborted = client.prepare(message, controller.signal);
    controller.abort();
    await assert.rejects(aborted, /cancelled/);
    workers[0][event]({ preventDefault() {} });
    assert.equal(workers[0].terminated, true);
    const next = client.prepare(message, new AbortController().signal);
    assert.deepEqual(workers[1].messages[0].snapshot, message.snapshot);
    workers[0][event]({ preventDefault() {} });
    assert.equal(
      workers[1].terminated,
      false,
      "old worker errors cannot reset its replacement",
    );
    workers[1].reply(2, []);
    assert.deepEqual(await next, []);
  }
});

test("worker timeouts and crashes clear pending work and allow fresh initialization", async () => {
  const workers = [];
  const client = new SceneWorkerClient(() => {
    const worker = new WorkerStub();
    workers.push(worker);
    return worker;
  }, 15);
  await assert.rejects(
    client.compute(message, new AbortController().signal),
    /timed out/,
  );
  const crash = client.compute(message, new AbortController().signal);
  workers[1].onerror({ preventDefault() {} });
  await assert.rejects(crash, /worker failed/);
  const retry = client.prepare(message, new AbortController().signal);
  workers[2].reply(workers[2].messages[0].id, []);
  assert.deepEqual(await retry, []);
});

for (const kind of ["prepare", "compute"]) {
  test(`${kind} error replies terminate the worker and retry with cached font bytes`, async () => {
    const workers = [];
    const client = new SceneWorkerClient(() => {
      const worker = new WorkerStub();
      workers.push(worker);
      return worker;
    });
    const failedController = new AbortController();
    const otherController = new AbortController();
    const failed = client[kind](message, failedController.signal);
    const other = client.compute(message, otherController.signal);
    const error = "Failed to fetch dynamically imported module";
    const rejected = Promise.all([
      assert.rejects(failed, { message: error }),
      assert.rejects(other, { message: error }),
    ]);
    workers[0].onmessage({ data: { id: workers[0].messages[0].id, error } });
    await rejected;
    assert.equal(workers[0].terminated, true);

    const prepared = client.prepare(message, new AbortController().signal);
    assert.equal(workers.length, 2);
    workers[1].reply(workers[1].messages[0].id, []);
    assert.deepEqual(await prepared, []);
    const retry = client.compute(message, new AbortController().signal);
    failedController.abort();
    otherController.abort();
    assert.equal(
      workers[1].terminated,
      false,
      "failed requests remove abort handlers",
    );
    assert.deepEqual(
      new Uint8Array(message.fonts[0].bytes),
      new Uint8Array([1, 2]),
    );
    assert.deepEqual(workers[1].messages[1].fonts, message.fonts);
    workers[1].reply(workers[1].messages[1].id, { recovered: true });
    assert.deepEqual(await retry, { recovered: true });
  });
}
