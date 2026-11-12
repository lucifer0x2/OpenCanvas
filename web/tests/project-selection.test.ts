import assert from "node:assert/strict";
import test from "node:test";
import { projectSelection } from "../src/projectSelection.ts";
import { EMPTY_SELECTION, mergeProjects, readSelection, reconcileProjects } from "../src/state.ts";
import { snapshot } from "./snapshot-http-support.ts";

const projects = [
  { projectId: "p", slug: "alpha", name: "Alpha", revision: 5 },
  { projectId: "q", slug: "beta", name: "Beta", revision: 2 },
];
const selection = { projectId: "p", pageId: "page", frameId: "frame", nodeId: "node" };
const accepted = snapshot({ nodes: [
  { id: "page", parentId: null, sortKey: 0, kind: "page", props: {} },
  { id: "frame", parentId: "page", sortKey: 0, kind: "frame", props: {} },
  { id: "node", parentId: "frame", sortKey: 0, kind: "rect", props: {} },
] });

test("no-op project refreshes retain both list and entry identity", () => {
  const incoming = structuredClone(projects);
  assert.equal(mergeProjects(projects, incoming), projects);
  assert.equal(reconcileProjects(projects, incoming), projects);
  assert.equal(reconcileProjects(projects, [{ ...incoming[0], revision: 1 }, incoming[1]]), projects);
  const renamed = reconcileProjects(projects, [{ ...incoming[0], name: "Renamed" }, incoming[1]]);
  assert.notEqual(renamed, projects);
  assert.equal(renamed[0].name, "Renamed");
  assert.equal(renamed[1], projects[1]);
  assert.deepEqual(reconcileProjects(projects, [incoming[1]]), [projects[1]]);
});

test("selection and fallback explanations transition atomically and repeated acceptance is a no-op", () => {
  const initial = { selection, notice: "" };
  assert.equal(projectSelection(initial, { type: "accepted", snapshot: accepted }), initial);
  assert.equal(projectSelection(initial, { type: "projects", projects }), initial);
  const removed = projectSelection(initial, {
    type: "accepted", snapshot: { ...accepted, nodes: accepted.nodes.slice(0, 2) },
  });
  assert.equal(removed.selection.nodeId, "frame");
  assert.equal(removed.notice, "The selected layer was removed. Showing its frame.");
  assert.equal(initial.selection.nodeId, "node", "the reducer does not mutate its input");
  const unavailable = projectSelection(initial, { type: "projects", projects: [projects[1]] });
  assert.equal(unavailable.selection.projectId, "q");
  assert.match(unavailable.notice, /unavailable/);
  const switched = projectSelection(unavailable, { type: "project", projectId: "p" });
  assert.equal(switched.notice, "");
  assert.equal(switched.selection.nodeId, "");
});

test("a foreign snapshot cannot clear a current catch-up notice; validated no-op clears it", () => {
  const state = { selection, notice: "Catching up with changes…" };
  assert.equal(projectSelection(state, {
    type: "accepted", snapshot: { ...accepted, projectId: "q" },
  }), state);
  const next = projectSelection(state, { type: "accepted", snapshot: accepted });
  assert.equal(next.selection, selection);
  assert.equal(next.notice, "");
});

test("stored selection only accepts string fields and excludes unrelated keys", (t) => {
  const old = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  Object.defineProperty(globalThis, "localStorage", { configurable: true, value: {
    getItem: () => JSON.stringify({ projectId: "p", pageId: 17, frameId: null, nodeId: {}, extra: "no" }),
  } });
  t.after(() => {
    if (old) Object.defineProperty(globalThis, "localStorage", old);
    else Reflect.deleteProperty(globalThis, "localStorage");
  });
  assert.deepEqual(readSelection(), { ...EMPTY_SELECTION, projectId: "p" });
});
