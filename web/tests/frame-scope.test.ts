import assert from "node:assert/strict";
import test from "node:test";
import { ALL_FRAMES, frameForNode } from "../src/frameScope.ts";
import { children, reconcileSelection } from "../src/state.ts";
import type { Selection, Snapshot } from "../src/types.ts";

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
test("All survives reload/revisions and keeps selection only inside this page's root frames", () => {
  const all = { ...selection, frameId: ALL_FRAMES, nodeId: "outside" };
  assert.deepEqual(reconcileSelection(snapshot, all), all);
  assert.equal(frameForNode(snapshot.nodes, "outside", "page")?.id, "other");
  assert.equal(
    reconcileSelection(snapshot, { ...all, nodeId: "deleted" }).nodeId,
    "",
  );
  const empty = reconcileSelection(
    {
      ...snapshot,
      nodes: snapshot.nodes.filter((node) => node.kind === "page"),
    },
    all,
  );
  assert.equal(empty.frameId, ALL_FRAMES);
  assert.equal(empty.nodeId, "");
  const changedPage = {
    ...snapshot,
    nodes: [
      ...snapshot.nodes,
      { id: "new-page", kind: "page", parentId: null, sortKey: 2, props: {} },
    ],
  };
  assert.equal(
    reconcileSelection(changedPage, { ...all, pageId: "new-page" }).nodeId,
    "",
  );
});

test("All cannot collide with an authored ID and ties follow the server's ASCII order", () => {
  assert.equal(/^[A-Za-z0-9_-]{1,128}$/.test(ALL_FRAMES), false);
  const root = snapshot.nodes.find((node) => node.id === "frame")!;
  const candidate = {
    ...snapshot,
    nodes: [
      ...snapshot.nodes,
      { ...root, id: "__all_frames__" },
      { ...root, id: "Z" },
      { ...root, id: "a" },
    ],
  };
  assert.equal(
    reconcileSelection(candidate, {
      ...selection,
      frameId: "__all_frames__",
      nodeId: "",
    }).frameId,
    "__all_frames__",
  );
  assert.deepEqual(
    children(candidate.nodes, "page")
      .filter((node) => node.sortKey === 0)
      .map((node) => node.id),
    ["Z", "__all_frames__", "a", "frame"],
  );
});
