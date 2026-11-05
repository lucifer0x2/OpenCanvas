import assert from "node:assert/strict";
import test from "node:test";
import { sceneModule } from "./scene-module.mjs";

const { SceneWorkerClient } = await sceneModule("workerClient");
const { sceneWorkerRuntime } = await sceneModule("workerRuntime");
const message = (revision = 1, projectId = "p") => ({
  request: { projectId, revision, frameId: "frame", width: 800, height: 600 },
  snapshot: { projectId, revision, nodes: [{ id: `revision-${revision}` }] },
  fonts: [{ id: "font", hash: "hash", bytes: new Uint8Array([1]).buffer }],
  urls: { fonts: {}, assets: {} },
});
const signal = () => new AbortController().signal;

function harness() {
  const workers = [],
    computers = [],
    calls = [];
  const client = new SceneWorkerClient(() => {
    const worker = {
      messages: [],
      terminated: false,
      terminate() {
        this.terminated = true;
      },
      postMessage(message) {
        this.messages.push(structuredClone(message));
        void handle(structuredClone(message));
      },
    };
    const handle = sceneWorkerRuntime(
      async () => {
        const computer = {
          freed: false,
          loads: 0,
          free() {
            this.freed = true;
          },
          load_font() {
            this.loads++;
          },
          set_resource_urls() {},
          required_fonts(json) {
            calls.push(json);
            return "[]";
          },
          compute_scene(json, frame, width, height) {
            calls.push(json);
            return JSON.stringify({
              ...JSON.parse(json),
              frame,
              width,
              height,
            });
          },
        };
        computers.push(computer);
        return computer;
      },
      (reply) => worker.onmessage({ data: reply }),
    );
    workers.push(worker);
    return worker;
  });
  return { client, workers, computers, calls };
}

test("prepare and viewport/frame recomputations transfer one structured snapshot and retain fonts", async () => {
  const { client, workers, computers, calls } = harness();
  await client.prepare(message(), signal());
  await client.compute(message(), signal());
  const next = message();
  next.request.frameId = "other";
  next.request.width = 390;
  const result = await client.compute(next, signal());
  assert.equal(result.frame, "other");
  assert.equal(result.width, 390);
  assert.deepEqual(
    workers[0].messages.map((item) => item.snapshot !== undefined),
    [true, false, false],
  );
  assert.ok(workers[0].messages.every((item) => !("snapshotJSON" in item)));
  assert.equal(new Set(calls).size, 1);
  assert.equal(computers.length, 1);
  assert.equal(computers[0].loads, 1);
  assert.deepEqual(workers[0].messages.map((item) => item.fonts.length), [1, 1, 0]);
});

test("new revisions are supplied once; project switch clears snapshots and font state", async () => {
  const { client, workers, computers } = harness();
  await client.prepare(message(), signal());
  await client.compute(message(2), signal());
  await client.compute(message(2), signal());
  await client.compute(message(2, "other"), signal());
  const original = await client.compute(message(2), signal());
  assert.equal(original.projectId, "p");
  assert.deepEqual(
    workers[0].messages.map((item) => item.snapshot !== undefined),
    [true, true, false, true, true],
  );
  assert.equal(computers.length, 3);
  assert.equal(computers[0].freed, true);
  assert.equal(computers[1].freed, true);
  assert.equal(computers[2].loads, 1);
});

test("aborted request leaves initialized WASM, cached snapshot and loaded fonts reusable", async () => {
  const { client, workers, computers } = harness();
  await client.compute(message(), signal());
  const controller = new AbortController();
  const obsolete = client.prepare(message(), controller.signal);
  const current = client.compute(message(), signal());
  controller.abort();
  await assert.rejects(obsolete, /cancelled/);
  assert.equal((await current).revision, 1);
  assert.equal((await client.compute(message(), signal())).revision, 1);
  assert.equal(workers.length, 1);
  assert.equal(workers[0].terminated, false);
  assert.equal(computers.length, 1);
  assert.equal(computers[0].loads, 1);
  assert.equal(workers[0].messages.filter((item) => item.snapshot).length, 1);
});

test("bounded revision cache resends evicted history and worker restart recovers during compute", async () => {
  const { client, workers, computers } = harness();
  for (let revision = 1; revision <= 5; revision++)
    await client.prepare(message(revision), signal());
  await client.compute(message(2), signal());
  assert.equal(workers[0].messages.at(-1).snapshot, undefined);
  const restored = await client.compute(message(), signal());
  assert.equal(restored.revision, 1);
  assert.ok(workers[0].messages.at(-1).snapshot);
  workers[0].onmessageerror();
  assert.equal(workers[0].terminated, true);
  const result = await client.compute(message(), signal());
  assert.equal(result.revision, 1);
  assert.ok(workers[1].messages[0].snapshot);
  assert.equal(computers.length, 2);
  assert.equal(computers[1].loads, 1);
});

test("Worker encodes structured snapshots once and rejects missing or mismatched references", async () => {
  let encodes = 0;
  const replies = [],
    inputs = [];
  const handle = sceneWorkerRuntime(
    async () => ({
      free() {},
      load_font() {},
      set_resource_urls() {},
      required_fonts(json) {
        inputs.push(json);
        return "[]";
      },
      compute_scene(json) {
        inputs.push(json);
        return "{}";
      },
    }),
    (reply) => { if (!("started" in reply)) replies.push(reply); },
  );
  const first = { ...message(), id: 1, kind: "prepare" };
  first.snapshot.toJSON = () => {
    encodes++;
    return message().snapshot;
  };
  await handle(first);
  const { snapshot, ...reference } = first;
  await handle({ ...reference, id: 2, kind: "compute" });
  assert.equal(encodes, 1);
  assert.equal(inputs[0], inputs[1]);
  await handle({
    ...reference,
    id: 3,
    request: { ...reference.request, revision: 99 },
  });
  await handle({
    ...first,
    id: 4,
    request: { ...first.request, projectId: "wrong" },
  });
  assert.match(replies[2].error, /snapshot cache is missing/);
  assert.match(replies[3].error, /snapshot cache is missing/);
});
