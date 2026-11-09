import assert from "node:assert/strict";
import test from "node:test";
import {
  SceneWorkerClient, WorkerStub, computer, harness, message, signal,
} from "./scene-worker-support.mjs";

test("acknowledged font id/hash is omitted until hash changes, project switches or reset", async () => {
  const { client, workers } = harness(async () => computer());
  await client.compute(message(), signal());
  await client.compute(message(2), signal());
  assert.equal(workers[0].messages[1].fonts.length, 0);
  const changed = message(2);
  changed.fonts[0].hash = "new-hash";
  await client.compute(changed, signal());
  assert.equal(workers[0].messages[2].fonts[0].hash, "new-hash");
  await client.compute(message(2, "other"), signal());
  await client.compute(message(2), signal());
  assert.equal(workers[0].messages[3].fonts.length, 1);
  assert.equal(workers[0].messages[4].fonts.length, 1);
  client.clearSnapshotCache();
  await client.compute(message(2), signal());
  assert.ok(workers[1].messages[0].snapshot);
  assert.equal(workers[1].messages[0].fonts.length, 1);
});

test("aborted successful requests still acknowledge fonts without transferring buffers", async () => {
  const worker = new WorkerStub();
  const client = new SceneWorkerClient(() => worker);
  const abort = new AbortController();
  const input = message();
  const obsolete = client.compute(input, abort.signal);
  worker.reply({ id: 1, started: "computing" });
  abort.abort();
  await assert.rejects(obsolete, /cancelled/);
  worker.reply({ id: 1, result: {}, registeredFonts: [{ id: "font", hash: "hash" }] });
  const next = client.compute(input, signal());
  assert.equal(worker.messages[1].fonts.length, 0);
  assert.equal(input.fonts[0].bytes.byteLength, 1);
  worker.reply({ id: 2, result: {} });
  await next;
});

test("unacknowledged and partially failed font loads are resent safely", async () => {
  let fail = true;
  const loaded = [];
  const { client, workers } = harness(async () => computer({
    load_font(id) {
      if (id === "second" && fail)
        throw { code: "INVALID_FONT", message: "bad font" };
      loaded.push(id);
    },
  }));
  const input = message();
  input.fonts.push({ id: "second", hash: "second-hash", bytes: new ArrayBuffer(1) });
  await assert.rejects(client.compute(input, signal()), { code: "INVALID_FONT" });
  fail = false;
  await client.compute(input, signal());
  await client.compute(input, signal());
  assert.deepEqual(workers[0].messages.map((item) => item.fonts.length), [2, 2, 0]);
  assert.deepEqual(loaded, ["font", "second"]);
});

for (const failure of ["compute", "partial font registration"]) {
  test(`acknowledged font bytes recover after ${failure} fails with a code`, async () => {
    const loaded = new Map();
    const registrations = [];
    const { client, workers } = harness(async () => computer({
      load_font(id, hash, bytes) {
        if (failure === "partial font registration" && id === "second" && hash === "B")
          throw { code: "INVALID_FONT", message: "second font rejected" };
        loaded.set(id, hash);
        registrations.push([id, hash, [...bytes]]);
      },
      compute_scene(snapshot) {
        const { revision } = JSON.parse(snapshot);
        if (revision === 2)
          throw { code: "LAYOUT_INVALID", message: "invalid layout after loading B" };
        if (loaded.get("font") !== "hash")
          throw { code: "RESOURCE_HASH_MISMATCH", message: "font A required" };
        return JSON.stringify({ fontHash: loaded.get("font") });
      },
    }));
    const original = message();
    if (failure === "partial font registration")
      original.fonts.push({ id: "second", hash: "hash", bytes: new Uint8Array([1]).buffer });
    await client.compute(original, signal());
    const changed = { ...message(2), fonts: original.fonts.map((font) => ({
      ...font, hash: "B", bytes: new Uint8Array([2]).buffer,
    })) };
    await assert.rejects(client.compute(changed, signal()), {
      code: failure === "compute" ? "LAYOUT_INVALID" : "INVALID_FONT",
    });
    const result = await client.compute({ ...message(3), fonts: original.fonts }, signal());
    assert.equal(result.fontHash, "hash");
    assert.equal(workers.length, 1);
    assert.equal(workers[0].terminated, false);
    assert.deepEqual(workers[0].messages.map((item) => item.fonts.length),
      Array(3).fill(original.fonts.length));
    assert.deepEqual(registrations.filter(([id]) => id === "font"), [
      ["font", "hash", [1]], ["font", "B", [2]], ["font", "hash", [1]],
    ]);
  });
}

test("late successful registration from another project cannot mark current fonts loaded", async () => {
  const worker = new WorkerStub();
  const client = new SceneWorkerClient(() => worker);
  const first = client.compute(message(), signal());
  const switched = client.compute(message(1, "other"), signal());
  worker.reply({ id: 1, result: {}, registeredFonts: [{ id: "font", hash: "hash" }] });
  await first;
  const current = client.compute(message(1, "other"), signal());
  assert.equal(worker.messages[2].fonts.length, 1);
  worker.reply({ id: 2, result: {}, registeredFonts: [{ id: "font", hash: "hash" }] });
  worker.reply({ id: 3, result: {}, registeredFonts: [{ id: "font", hash: "hash" }] });
  await Promise.all([switched, current]);
});

test("unacknowledged font hash changes cannot make concurrent requests omit required bytes", async () => {
  const worker = new WorkerStub();
  const client = new SceneWorkerClient(() => worker);
  const warm = client.compute(message(), signal());
  worker.reply({ id: 1, result: {}, registeredFonts: [{ id: "font", hash: "hash" }] });
  await warm;
  const changed = message(2);
  changed.fonts[0].hash = "changed";
  const first = client.compute(changed, signal());
  const second = client.compute(message(3), signal());
  assert.equal(worker.messages[2].fonts[0].hash, "hash");
  worker.reply({ id: 2, result: {}, registeredFonts: [{ id: "font", hash: "changed" }] });
  worker.reply({ id: 3, result: {}, registeredFonts: [{ id: "font", hash: "hash" }] });
  await Promise.all([first, second]);
});
