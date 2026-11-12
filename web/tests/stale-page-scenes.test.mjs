import assert from "node:assert/strict";
import test from "node:test";
import { sceneModule } from "./scene-module.mjs";
import { sceneFor } from "./scene-fixtures.mjs";

const { seedFrames, reuseFrame, failFrame, pageView, currentScene, shownRevision } =
  await sceneModule("staleFrames");
const { browserPagePass } = await sceneModule("browserPage");
const { cachedBrowserScene } = await sceneModule("browserSceneCache");

const target = (id) => ({ id, name: id, width: 800, height: 600 });
const request = (revision, id) => ({
  projectId: "p",
  revision,
  frameId: id,
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
const signal = () => new AbortController().signal;

/** The page hook's loop: seed from the last view, reuse equal keys, compute the rest. */
function page(digests) {
  const computed = [];
  const loaded = [];
  const compute = async (desired) => {
    computed.push(`${desired.frameId}@${desired.revision}`);
    return { job: { ...desired, status: "ready", scene: sceneFor(desired) } };
  };
  const browser = {
    keys: async (requests) =>
      new Map(requests.map((r) => [r.frameId, digests[r.revision][r.frameId]])),
    load: (r, s, sig, bypass, digest) => {
      loaded.push(r.frameId);
      return cachedBrowserScene(r, s, sig, compute, bypass, digest);
    },
  };
  const render = async (revision, ids, previous, abort = signal()) => {
    const targets = ids.map(target);
    const entries = previous
      ? seedFrames(targets, previous)
      : targets.map((t) => ({ ...t }));
    const seededView = pageView(entries, { seeded: Boolean(previous) });
    const index = (id) => targets.findIndex((t) => t.id === id);
    const pass = await browserPagePass(
      ids.map((id) => request(revision, id)),
      snapshot(revision),
      abort,
      {
        reuse: (r, key) => {
          const reused = reuseFrame(entries[index(r.frameId)], r, key);
          if (reused) entries[index(r.frameId)] = reused;
          return Boolean(reused);
        },
        onFrame: async (result) => {
          entries[index(result.request.frameId)] = {
            ...targets[index(result.request.frameId)],
            scene: result.job.scene,
            resources: {},
            revision: result.request.revision,
            contentKey: result.contentKey,
          };
          result.persist?.();
        },
      },
      browser,
    );
    assert.equal(pass.fallback, "");
    return { entries, seededView, view: pageView(entries, { finished: true }) };
  };
  return { computed, loaded, render };
}

// Digests are unique per test: the cache module keeps one memory tier for this file.
const d = (c) => c.repeat(64);

test("a new revision keeps every rendered frame and recomputes only changed or added frames", async () => {
  const { computed, loaded, render } = page({
    7: { a: d("1"), b: d("2"), c: d("3") },
    8: { a: d("1"), b: d("9"), d: d("4") },
  });
  const first = await render(7, ["a", "b", "c"]);
  assert.deepEqual(computed, ["a@7", "b@7", "c@7"]);
  loaded.length = 0;
  const next = await render(8, ["a", "b", "d"], first.entries);
  // Before any work: rendered frames stay on screen, the added one is a placeholder.
  assert.equal(next.seededView.status, "pending");
  assert.equal(next.seededView.revalidating, true);
  assert.deepEqual(
    next.seededView.frames.map((f) => Boolean(f.scene)),
    [true, true, false],
  );
  assert.deepEqual(
    next.seededView.frames.map((f) => f.id),
    ["a", "b", "d"],
    "removed frame is gone",
  );
  assert.deepEqual(next.seededView.progress, { ready: 0, total: 3 });
  // Only the changed and the added frame were loaded or computed.
  assert.deepEqual(loaded, ["b", "d"]);
  assert.deepEqual(computed, ["a@7", "b@7", "c@7", "b@8", "d@8"]);
  assert.equal(next.entries[0].revision, 8, "reused scene is current");
  assert.equal(
    next.entries[0].scene,
    first.entries[0].scene,
    "reuse keeps the scene object, so indexes and frame bitmaps hold",
  );
  assert.equal(next.entries[0].scene.provenance.revision, 7);
  assert.equal(currentScene(next.entries[0], 8), first.entries[0].scene);
  assert.equal(currentScene(next.entries[0], 7), undefined);
  assert.equal(next.view.sceneRevision, 8);
  assert.equal(shownRevision(next.view), 8);
  assert.equal(next.entries[1].revision, 8, "recomputed frame answers r8");
  assert.equal(next.entries[1].scene.provenance.revision, 8);
  assert.equal(
    next.entries[0].resources,
    first.entries[0].resources,
    "reuse keeps loaded resources",
  );
  assert.ok(next.entries.every((e) => !e.stale));
  assert.equal(next.view.status, "ready");
  assert.equal(next.view.revalidating, false);
});

test("a seeded frame keeps its last revision but is never current until revalidated", () => {
  const scene = sceneFor(request(4, "a"));
  const [stale] = seedFrames(
    [target("a")],
    [{ ...target("a"), scene, resources: {}, revision: 4, contentKey: "k" }],
  );
  assert.equal(stale.revision, 4);
  assert.equal(currentScene(stale, 4), undefined, "stale scenes are not hit or inspected");
  assert.equal(currentScene(stale, 5), undefined);
  const reused = reuseFrame(stale, request(5, "a"), "k");
  assert.equal(reused.scene, scene);
  assert.equal(reused.resources, stale.resources);
  assert.equal(currentScene(reused, 5), scene);
  assert.equal(currentScene(reused, undefined), undefined);
});

test("a fresh view of an unchanged frame at a later revision is served from memory", async () => {
  const { computed, loaded, render } = page({
    1: { a: d("5"), b: d("6") },
    2: { a: d("5"), b: d("7") },
  });
  await render(1, ["a", "b"]);
  const fresh = await render(2, ["a", "b"]);
  assert.deepEqual(
    loaded,
    ["a", "b", "a", "b"],
    "no previous view: both go through the cache",
  );
  assert.deepEqual(computed, ["a@1", "b@1", "b@2"]);
  assert.equal(fresh.entries[0].scene.provenance.revision, 2);
});

test("an aborted older revision publishes nothing after the newer one starts", async () => {
  const { render } = page({ 3: { a: d("8") } });
  const controller = new AbortController();
  controller.abort();
  await assert.rejects(render(3, ["a"], undefined, controller.signal));
});

test("failed revalidation keeps the last scene and reports it quietly", () => {
  const scene = sceneFor(request(4, "a"));
  const [stale] = seedFrames(
    [target("a")],
    [{ ...target("a"), scene, resources: {}, contentKey: "k" }],
  );
  assert.equal(reuseFrame(stale, request(5, "a"), "other"), undefined);
  assert.equal(
    reuseFrame({ ...stale, stale: false }, request(5, "a"), "k"),
    undefined,
  );
  const failed = failFrame(stale, target("a"), "boom");
  assert.equal(failed.scene, scene);
  assert.equal(
    failed.error,
    undefined,
    "no error text is painted over the frame",
  );
  assert.equal(failed.updateError, "boom");
  const view = pageView([failed], { finished: true, seeded: true });
  assert.equal(view.status, "failed");
  assert.match(view.error, /1 of 1 frames failed/);
  assert.equal(view.frames[0].scene, scene);
  const blank = failFrame(target("b"), target("b"), "boom");
  assert.equal(blank.error, "boom");
  const whole = pageView(seedFrames([target("a")], [failed]), {
    finished: true,
    error: "offline",
    seeded: true,
  });
  assert.equal(whole.status, "failed");
  assert.equal(
    whole.frames[0].scene,
    scene,
    "a failed page keeps earlier scenes",
  );
  assert.equal(whole.frames[0].updateError, undefined);
});
