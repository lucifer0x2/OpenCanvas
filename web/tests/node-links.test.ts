import assert from "node:assert/strict";
import test from "node:test";
import { nodeLink, readNodeLink, selectionNode } from "../src/deepLink.ts";
import { readSelection, reconcileSelection } from "../src/state.ts";
import type { Snapshot, Selection } from "../src/types.ts";
import { frameForNode } from "../src/frameScope.ts";
import { nodeIndex } from "../src/nodeIndex.ts";

const selection: Selection = {
  projectId: "p",
  pageId: "page",
  frameId: "frame",
  nodeId: "instance/label",
};
const snapshot = {
  projectId: "p",
  nodes: [
    { id: "page", parentId: null, kind: "page", sortKey: 0, props: {} },
    { id: "frame", parentId: "page", kind: "frame", sortKey: 0, props: {} },
    {
      id: "master",
      parentId: "page",
      kind: "frame",
      sortKey: 1,
      props: { component: { name: "Card" } },
    },
    {
      id: "label",
      parentId: "master",
      kind: "text",
      sortKey: 0,
      props: { text: "Master" },
    },
    {
      id: "instance",
      parentId: "frame",
      kind: "instance",
      sortKey: 0,
      props: {
        componentId: "master",
        overrides: { label: { text: "Placed" } },
      },
    },
  ],
} as unknown as Snapshot;

test("stable addresses encode virtual IDs and omit viewport/revision/cache", () => {
  const link = nodeLink(
    "http://127.0.0.1:9876/preview?width=500&revision=3#old",
    selection,
  );
  assert.equal(
    link,
    "http://127.0.0.1:9876/?project=p&page=page&frame=frame&node=instance%2Flabel",
  );
  assert.deepEqual(readNodeLink(link), selection);
  assert.equal(readNodeLink("http://localhost/?project=p&node=label"), null);
});

test("linked selection overrides local storage and virtual children survive reconciliation", () => {
  Object.defineProperty(globalThis, "location", {
    configurable: true,
    value: { href: nodeLink("http://localhost", selection) },
  });
  Object.defineProperty(globalThis, "localStorage", {
    configurable: true,
    value: { getItem: () => JSON.stringify({ ...selection, nodeId: "wrong" }) },
  });
  assert.deepEqual(readSelection(), selection);
  assert.deepEqual(reconcileSelection(snapshot, selection), selection);
  assert.equal(reconcileSelection(snapshot, selection), selection);
  assert.equal(frameForNode(snapshot.nodes, selection.nodeId, "page")?.id, "frame");
  assert.equal(nodeIndex(snapshot.nodes).byId.has(selection.nodeId), false);
  assert.equal(
    selectionNode(snapshot.nodes, selection.nodeId)?.props.text,
    "Placed",
  );
  assert.equal(selectionNode(snapshot.nodes, "instance/nope"), undefined);
  assert.equal(
    reconcileSelection(snapshot, { ...selection, nodeId: "instance/nope" })
      .nodeId,
    "frame",
  );
});

test("indexed resolution rejects unrelated master children and ancestry cycles", () => {
  const nodes = [...snapshot.nodes,
    { id: "unrelated", parentId: "frame", kind: "text", sortKey: 1, props: {} },
    { id: "cycle-a", parentId: "cycle-b", kind: "group", sortKey: 0, props: {} },
    { id: "cycle-b", parentId: "cycle-a", kind: "group", sortKey: 0, props: {} },
  ];
  assert.equal(selectionNode(nodes, "instance/unrelated"), undefined);
  assert.equal(selectionNode(nodes, "instance/cycle-a"), undefined);
  assert.equal(frameForNode(nodes, "cycle-a", "page"), undefined);
  assert.equal(frameForNode(nodes, selection.nodeId, "other-page"), undefined);
});

test("a virtual descendant follows generated parent IDs back to its placed frame", () => {
  const nodes = [
    ...snapshot.nodes,
    { id: "group", parentId: "master", kind: "group", sortKey: 1, props: {} },
    { id: "nested", parentId: "group", kind: "text", sortKey: 0, props: {} },
  ];
  const virtualId = "instance/nested";
  assert.equal(selectionNode(nodes, virtualId)?.parentId, "instance/group");
  assert.equal(frameForNode(nodes, virtualId, "page")?.id, "frame");
  const placed = { ...selection, nodeId: virtualId };
  assert.equal(reconcileSelection({ ...snapshot, nodes }, placed), placed);
});

test("explicit deselection survives a fresh snapshot and SSE reconciliation", () => {
  const empty = { ...selection, nodeId: "" };
  assert.deepEqual(reconcileSelection(snapshot, empty), empty);
  assert.deepEqual(
    reconcileSelection({ ...snapshot, revision: 9 }, empty),
    empty,
  );
  assert.equal(
    reconcileSelection(snapshot, { ...selection, nodeId: "deleted" }).nodeId,
    "frame",
  );
});
