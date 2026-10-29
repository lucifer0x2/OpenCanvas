import assert from "node:assert/strict";
import test from "node:test";
import { nodeLink, readNodeLink, selectionNode } from "../src/deepLink.ts";
import { readSelection, reconcileSelection } from "../src/state.ts";
import type { Snapshot, Selection } from "../src/types.ts";

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
