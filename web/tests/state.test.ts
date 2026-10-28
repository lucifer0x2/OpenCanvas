import assert from "node:assert/strict";
import test from "node:test";
import {
  acceptsJob,
  bumpRevision,
  children,
  clearActivity,
  commitAction,
  followTarget,
  markActivity,
  mergeProjects,
  pickProject,
  reconcileSelection,
} from "../src/state.ts";
import type {
  ProjectRef,
  RenderJob,
  SceneRequest,
  Selection,
  Snapshot,
} from "../src/types.ts";

const snapshot = {
  projectId: "p",
  revision: 3,
  nodes: [
    { id: "page", parentId: null, kind: "page", sortKey: 0, props: {} },
    { id: "frame", parentId: "page", kind: "frame", sortKey: 0, props: {} },
    { id: "text", parentId: "frame", kind: "text", sortKey: 0, props: {} },
    { id: "other", parentId: "page", kind: "frame", sortKey: 1, props: {} },
    { id: "outside", parentId: "other", kind: "text", sortKey: 0, props: {} },
  ],
} as Snapshot;
const selection: Selection = {
  projectId: "p",
  pageId: "page",
  frameId: "frame",
  nodeId: "text",
};
const request: SceneRequest = {
  projectId: "p",
  revision: 3,
  frameId: "frame",
  width: 800,
  height: 600,
};
const job = (): RenderJob => ({
  jobId: "j3",
  status: "ready",
  projectId: "p",
  revision: 3,
  frameId: "frame",
  sceneKey: "key3",
  scene: {
    schemaVersion: 1,
    coordinateSpace: "frame-local",
    nodes: [],
    fonts: [],
    assets: [],
    diagnostics: [],
    provenance: {
      projectId: "p",
      revision: 3,
      frameId: "frame",
      viewport: { width: 800, height: 600 },
      theme: "default",
      state: "default",
      sourceSceneKey: "key3",
      sceneVersion: "scene-2",
      rendererVersion: "web-2",
      fontManifestHash: "",
      assetManifestHash: "",
    },
  },
});

test("layers use storage's numeric sort keys", () => {
  const ordered = children(
    snapshot.nodes.map((node) => ({
      ...node,
      sortKey:
        node.id === "frame" ? 10 : node.id === "other" ? 2 : node.sortKey,
    })),
    "page",
  );
  assert.deepEqual(
    ordered.map((node) => node.id),
    ["other", "frame"],
  );
});

test("reload keeps valid selection; deleted frame or layer gets a valid fallback", () => {
  assert.deepEqual(reconcileSelection(snapshot, selection), selection);
  assert.equal(
    reconcileSelection(snapshot, { ...selection, nodeId: "deleted" }).nodeId,
    "frame",
  );
  assert.equal(
    reconcileSelection(snapshot, { ...selection, nodeId: "outside" }).nodeId,
    "frame",
  );
  assert.equal(
    reconcileSelection(snapshot, { ...selection, frameId: "deleted" }).frameId,
    "frame",
  );
  assert.equal(
    reconcileSelection({ ...snapshot, nodes: [] }, selection).frameId,
    "",
  );
});

test("SSE duplicates and foreign projects are ignored; gaps require snapshot reconciliation", () => {
  assert.equal(commitAction({ projectId: "p", revision: 3 }, "p", 3), "ignore");
  assert.equal(commitAction({ projectId: "q", revision: 4 }, "p", 3), "ignore");
  assert.equal(
    commitAction({ projectId: "p", revision: 4 }, "p", 3),
    "refresh",
  );
  assert.equal(commitAction({ projectId: "p", revision: 7 }, "p", 3), "gap");
  assert.equal(
    commitAction({ projectId: "p", revision: NaN }, "p", 3),
    "ignore",
  );
});

test("late jobs, mislabeled scene provenance, viewport races and wrong job IDs are rejected", () => {
  assert.equal(acceptsJob(job(), request, "j3"), true);
  assert.equal(acceptsJob(job(), { ...request, revision: 4 }), false);
  assert.equal(acceptsJob(job(), request, "j4"), false);
  assert.equal(acceptsJob(job(), { ...request, width: 390 }), false);
  const mislabeled = job();
  mislabeled.scene!.provenance.revision = 2;
  assert.equal(acceptsJob(mislabeled, request), false);
  const wrongKey = job();
  wrongKey.sceneKey = "different-key";
  assert.equal(acceptsJob(wrongKey, request), false);
});

const ref = (projectId: string, slug: string, revision = 0): ProjectRef => ({
  projectId,
  slug,
  name: slug,
  revision,
});

test("project_created merges by projectId in slug order without moving revisions back", () => {
  const list = [ref("a", "alpha", 2), ref("c", "gamma")];
  const created = mergeProjects(list, [ref("b", "beta")]);
  assert.deepEqual(
    created.map((entry) => entry.slug),
    ["alpha", "beta", "gamma"],
  );
  assert.deepEqual(mergeProjects(created, [ref("b", "beta")]), created);
  // A stale list response cannot drop a hinted project or lower a revision.
  const stale = mergeProjects(created, [ref("a", "alpha", 1)]);
  assert.equal(stale.length, 3);
  assert.equal(stale[0].revision, 2);
  assert.equal(mergeProjects([], [ref("z", "z", 4)])[0].revision, 4);
});

test("project_activity bumps only a listed project forward", () => {
  const list = [ref("a", "alpha", 2), ref("b", "beta", 1)];
  const bumped = bumpRevision(list, { projectId: "b", revision: 5 });
  assert.equal(bumped[1].revision, 5);
  assert.equal(bumped[0], list[0]);
  assert.equal(bumpRevision(list, { projectId: "b", revision: 1 }), list);
  assert.equal(bumpRevision(list, { projectId: "x", revision: 9 }), list);
  assert.equal(bumpRevision(list, { projectId: "b", revision: NaN }), list);
});

test("activity marks other projects once and clears on selection", () => {
  let marked = markActivity([], "b", "a");
  assert.deepEqual(marked, ["b"]);
  assert.equal(markActivity(marked, "b", "a"), marked);
  assert.equal(markActivity(marked, "a", "a"), marked);
  marked = markActivity(marked, "c", "a");
  assert.deepEqual(clearActivity(marked, "b"), ["c"]);
  assert.equal(clearActivity(marked, "z"), marked);
});

test("project fallback and follow-agent targets", () => {
  const list = [ref("a", "alpha"), ref("b", "beta")];
  assert.equal(pickProject(list, "b"), "b");
  assert.equal(pickProject(list, "gone"), "a");
  assert.equal(pickProject([], "gone"), "");
  // An empty workspace always takes the first created project.
  assert.equal(followTarget(false, "a", ""), "a");
  // Otherwise only the follow switch moves away from the project in view.
  assert.equal(followTarget(false, "b", "a"), null);
  assert.equal(followTarget(true, "b", "a"), "b");
  assert.equal(followTarget(true, "a", "a"), null);
  assert.equal(followTarget(true, "", "a"), null);
});
