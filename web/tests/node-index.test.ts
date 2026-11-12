import assert from "node:assert/strict";
import test from "node:test";
import { nodeIndex, snapshotNodeIndex } from "../src/nodeIndex.ts";
import { children, reconcileSelection } from "../src/state.ts";
import { frameForNode } from "../src/frameScope.ts";
import { selectionNode } from "../src/deepLink.ts";
import { layerSelectionPath } from "../src/layerSelection.ts";
import type { DesignNode, Snapshot } from "../src/types.ts";

const nodes = (): DesignNode[] => [
  { id: "page", parentId: null, kind: "page", sortKey: 0, props: {} },
  { id: "frame", parentId: "page", kind: "frame", sortKey: 0, props: {} },
  { id: "a", parentId: "frame", kind: "text", sortKey: 0, props: { text: "old" } },
  { id: "b", parentId: "frame", kind: "rect", sortKey: 1, props: {} },
];
const snapshot = (projectId: string, revision = 1) => ({ projectId, revision, nodes: nodes() });

test("project/revision owns the index across cloned JSON arrays and project switches", () => {
  const a = snapshot("cache-a");
  const first = snapshotNodeIndex(a);
  const clone = structuredClone(a);
  let visits = 0;
  Object.defineProperty(clone.nodes[0], "id", { get() { visits++; return "page"; } });
  assert.equal(snapshotNodeIndex(clone), first);
  assert.equal(nodeIndex(clone.nodes), first);
  assert.equal(visits, 0, "matching revision never traverses the replacement nodes");
  const b = snapshot("cache-b");
  b.nodes[2].props.text = "other project";
  const second = snapshotNodeIndex(b);
  assert.notEqual(second, first);
  assert.equal(second.byId.get("a")?.props.text, "other project");
  // Visiting many projects must not evict another project's newest revision.
  for (let i = 0; i < 100; i++) snapshotNodeIndex(snapshot(`visited-${i}`));
  assert.equal(snapshotNodeIndex(structuredClone(a)), first);
});

test("new revisions replace props and sorted children without mutating old indexes", () => {
  const a = snapshot("cache-updated");
  const first = snapshotNodeIndex(a);
  const next = structuredClone(a);
  next.revision++;
  next.nodes[3].sortKey = -1;
  next.nodes[2].props.text = "new";
  const updated = snapshotNodeIndex(next);
  assert.notEqual(updated, first);
  assert.deepEqual(updated.childrenByParent.get("frame")?.map((node) => node.id), ["b", "a"]);
  assert.equal(updated.byId.get("a")?.props.text, "new");
  assert.equal(first.byId.get("a")?.props.text, "old");
  assert.deepEqual(first.childrenByParent.get("frame")?.map((node) => node.id), ["a", "b"]);
  // A delayed old render and even a newly parsed backwards snapshot cannot downgrade it.
  assert.equal(snapshotNodeIndex(a), first);
  const stale = structuredClone(a);
  assert.notEqual(snapshotNodeIndex(stale), updated);
  assert.equal(snapshotNodeIndex(structuredClone(next)), updated);
  const nextRevision = snapshotNodeIndex({ ...next, revision: next.revision + 1 });
  assert.notEqual(nextRevision, updated, "a revision change rebuilds even if the nodes array is shared");
});

test("primed refetch arrays share authored indexes throughout array-only selection helpers", () => {
  const a = snapshot("cache-helpers");
  const first = snapshotNodeIndex(a);
  const refetch = structuredClone(a);
  snapshotNodeIndex(refetch);
  // All helpers must use the primed original nodes, never scan the refetched array.
  Object.defineProperty(refetch.nodes, Symbol.iterator, { value() { throw new Error("extra index build"); } });
  assert.equal(nodeIndex(refetch.nodes), first);
  assert.equal(children(refetch.nodes, "frame")[0], a.nodes[2]);
  assert.equal(selectionNode(refetch.nodes, "a"), a.nodes[2]);
  assert.equal(frameForNode(refetch.nodes, "a", "page"), a.nodes[1]);
  assert.deepEqual(layerSelectionPath(refetch.nodes, "a"), { id: "frame", child: { id: "a", child: undefined } });
  const selection = { projectId: a.projectId, pageId: "page", frameId: "frame", nodeId: "a" };
  assert.equal(reconcileSelection(refetch as Snapshot, selection), selection);
});
