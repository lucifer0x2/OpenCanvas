import assert from "node:assert/strict";
import test from "node:test";
import {
  acceptsJob,
  children,
  commitAction,
  reconcileSelection,
} from "../src/state.ts";
import type {
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
      sceneVersion: "scene-1",
      rendererVersion: "web-1",
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
