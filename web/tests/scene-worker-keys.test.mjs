import assert from "node:assert/strict";
import test from "node:test";
import { computer, harness, message, signal } from "./scene-worker-support.mjs";

const frames = [{ frameId: "frame", width: 800, height: 600 }];
const keyed = (revision = 1) => ({ ...message(revision), fonts: [], frames });

test("keys share the revision's single snapshot transfer with later computes", async () => {
  const calls = [];
  const { client, workers } = harness(async () =>
    computer({
      frame_keys(snapshot, list, theme) {
        calls.push([JSON.parse(snapshot).revision, JSON.parse(list), theme]);
        return JSON.stringify([{ frameId: "frame", digest: "f".repeat(64) }]);
      },
    }),
  );
  assert.deepEqual(await client.keys(keyed(), signal()), [
    { frameId: "frame", digest: "f".repeat(64) },
  ]);
  await client.compute(message(), signal());
  await client.keys(keyed(2), signal());
  assert.deepEqual(calls, [
    [1, frames, "default"],
    [2, frames, "default"],
  ]);
  assert.deepEqual(
    workers[0].messages.map((item) => [item.kind, item.snapshot !== undefined]),
    [
      ["keys", true],
      ["compute", false],
      ["keys", true],
    ],
  );
});

test("a scene module without frame_keys answers with a coded error and keeps the worker", async () => {
  const { client, workers } = harness(async () => computer());
  await assert.rejects(
    client.keys(keyed(), signal()),
    (error) => error.code === "KEYS_UNAVAILABLE",
  );
  assert.deepEqual(await client.prepare(message(), signal()), []);
  assert.equal(workers.length, 1);
  assert.equal(workers[0].terminated, false);
});
