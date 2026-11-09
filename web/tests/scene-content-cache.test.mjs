import assert from "node:assert/strict";
import test from "node:test";
import { sceneModule } from "./scene-module.mjs";
import { sceneFor } from "./scene-fixtures.mjs";

const { cachedBrowserScene } = await sceneModule("browserSceneCache");
const { browserFrameKeys } = await sceneModule("frameKeys");
const { frameContentKey, rebaseScene } = await sceneModule("sceneIdentity");

const digest = (char) => char.repeat(64);
const request = (revision, frameId = "frame") => ({
  projectId: "p",
  revision,
  frameId,
  width: 800,
  height: 600,
});
const snapshot = (revision) => ({
  projectId: "p",
  revision,
  truncated: false,
  nodes: [],
  tokens: [],
  styles: [],
  fonts: [],
  assets: [],
});
const counter = () => {
  const calls = [];
  const compute = async (desired) => {
    calls.push(desired.revision);
    return { job: { ...desired, status: "ready", scene: sceneFor(desired) } };
  };
  return { calls, compute };
};
const signal = () => new AbortController().signal;

test("an unchanged content key at a new revision is a memory hit, restamped to that revision", async () => {
  const { calls, compute } = counter();
  const first = await cachedBrowserScene(
    request(7),
    snapshot(7),
    signal(),
    compute,
    false,
    digest("a"),
  );
  first.persist();
  const next = await cachedBrowserScene(
    request(8),
    snapshot(8),
    signal(),
    compute,
    false,
    digest("a"),
  );
  assert.deepEqual(calls, [7], "no recomputation for equal content");
  assert.equal(next.job.revision, 8);
  assert.equal(next.job.scene.provenance.revision, 8);
  assert.equal(next.job.sceneKey, next.job.scene.provenance.sourceSceneKey);
  await cachedBrowserScene(
    request(9),
    snapshot(9),
    signal(),
    compute,
    false,
    digest("b"),
  );
  assert.deepEqual(calls, [7, 9], "changed content recomputes");
});

test("no digest means no cache; bypass recomputes; unpersisted scenes are not remembered", async () => {
  const { calls, compute } = counter();
  await cachedBrowserScene(
    request(1),
    snapshot(1),
    signal(),
    compute,
    false,
    "",
  );
  (
    await cachedBrowserScene(
      request(2),
      snapshot(2),
      signal(),
      compute,
      false,
      "",
    )
  ).persist();
  await cachedBrowserScene(
    request(3),
    snapshot(3),
    signal(),
    compute,
    false,
    "",
  );
  assert.deepEqual(calls, [1, 2, 3]);
  await cachedBrowserScene(
    request(4),
    snapshot(4),
    signal(),
    compute,
    false,
    digest("c"),
  );
  await cachedBrowserScene(
    request(5),
    snapshot(5),
    signal(),
    compute,
    false,
    digest("c"),
  );
  assert.deepEqual(
    calls,
    [1, 2, 3, 4, 5],
    "results whose resources never became ready are not reused",
  );
  (
    await cachedBrowserScene(
      request(6),
      snapshot(6),
      signal(),
      compute,
      false,
      digest("c"),
    )
  ).persist();
  await cachedBrowserScene(
    request(7),
    snapshot(7),
    signal(),
    compute,
    true,
    digest("c"),
  );
  assert.deepEqual(calls, [1, 2, 3, 4, 5, 6, 7], "retry bypasses memory");
});

test("older results never stand in for a newer request", async () => {
  const stale = async (desired) => ({
    job: {
      ...desired,
      status: "ready",
      scene: sceneFor({ ...desired, revision: desired.revision - 1 }),
    },
  });
  await assert.rejects(
    cachedBrowserScene(
      request(8),
      snapshot(8),
      signal(),
      stale,
      false,
      digest("d"),
    ),
    /does not match this revision/,
    "a computed scene is validated, never restamped",
  );
  const { compute } = counter();
  await assert.rejects(
    cachedBrowserScene(
      request(8),
      snapshot(7),
      signal(),
      compute,
      false,
      digest("d"),
    ),
    /accepted snapshot/,
  );
  // Restamping only ever moves an equivalent scene to the request it answers.
  const scene = sceneFor(request(7));
  assert.equal(rebaseScene(scene, request(7)), scene);
  assert.equal(rebaseScene(scene, request(9)).provenance.revision, 9);
  assert.equal(scene.provenance.revision, 7);
});

test("content keys require a digest and separate viewports, frames and projects", () => {
  assert.equal(frameContentKey(request(1), ""), "");
  assert.equal(frameContentKey(request(1), "short"), "");
  const key = frameContentKey(request(1), digest("e"));
  assert.equal(
    frameContentKey(request(2), digest("e")),
    key,
    "revision is not part of the key",
  );
  for (const other of [
    { ...request(1), width: 390 },
    { ...request(1), frameId: "other" },
    { ...request(1), projectId: "q" },
  ])
    assert.notEqual(frameContentKey(other, digest("e")), key);
});

test("frame keys: one batched call, positional digests, coded errors mean no cache", async () => {
  const requests = [request(4, "a"), request(4, "b"), request(4, "c")];
  const messages = [];
  const worker = (reply) => ({
    keys: async (message) => {
      messages.push(message);
      if (reply instanceof Error) throw reply;
      return reply;
    },
  });
  const keys = await browserFrameKeys(
    requests,
    snapshot(4),
    signal(),
    worker([
      { frameId: "a", digest: digest("1") },
      { frameId: "b", error: { code: "INVALID_REQUEST", message: "budget" } },
      { frameId: "c", digest: "not-a-digest" },
    ]),
  );
  assert.deepEqual([...keys], [["a", digest("1")]]);
  assert.equal(messages.length, 1);
  assert.deepEqual(
    messages[0].frames.map((f) => f.frameId),
    ["a", "b", "c"],
  );
  assert.equal(messages[0].snapshot.revision, 4);
  const misordered = await browserFrameKeys(
    requests,
    snapshot(4),
    signal(),
    worker([
      { frameId: "b", digest: digest("1") },
      { frameId: "a", digest: digest("2") },
      { frameId: "c", digest: digest("3") },
    ]),
  );
  assert.deepEqual([...misordered], [["c", digest("3")]]);
  const coded = Object.assign(new Error("old module"), {
    code: "KEYS_UNAVAILABLE",
  });
  assert.equal(
    (await browserFrameKeys(requests, snapshot(4), signal(), worker(coded)))
      .size,
    0,
  );
  await assert.rejects(
    browserFrameKeys(
      requests,
      snapshot(4),
      signal(),
      worker(new Error("worker failed")),
    ),
    /worker failed/,
  );
  await assert.rejects(
    browserFrameKeys(requests, snapshot(3), signal(), worker([])),
    /accepted snapshot/,
  );
});
