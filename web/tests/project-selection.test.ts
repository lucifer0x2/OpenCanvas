import assert from "node:assert/strict";
import test from "node:test";
import { readNavigation } from "../src/deepLink.ts";
import { ALL_FRAMES } from "../src/frameScope.ts";
import { projectSelection } from "../src/projectSelection.ts";
import { mergeProjects, reconcileProjects } from "../src/state.ts";
import { snapshot } from "./snapshot-http-support.ts";

const projects = [
  { projectId: "p", slug: "alpha", name: "Alpha", revision: 5 },
  { projectId: "q", slug: "beta", name: "Beta", revision: 2 },
];
const selection = {
  projectId: "p",
  pageId: "page",
  frameId: "frame",
  nodeId: "node",
};
const accepted = snapshot({
  nodes: [
    { id: "page", parentId: null, sortKey: 0, kind: "page", props: {} },
    { id: "frame", parentId: "page", sortKey: 0, kind: "frame", props: {} },
    { id: "node", parentId: "frame", sortKey: 0, kind: "rect", props: {} },
  ],
});

test("no-op project refreshes retain both list and entry identity", () => {
  const incoming = structuredClone(projects);
  assert.equal(mergeProjects(projects, incoming), projects);
  assert.equal(reconcileProjects(projects, incoming), projects);
  assert.equal(
    reconcileProjects(projects, [{ ...incoming[0], revision: 1 }, incoming[1]]),
    projects,
  );
  const renamed = reconcileProjects(projects, [
    { ...incoming[0], name: "Renamed" },
    incoming[1],
  ]);
  assert.notEqual(renamed, projects);
  assert.equal(renamed[0].name, "Renamed");
  assert.equal(renamed[1], projects[1]);
  assert.deepEqual(reconcileProjects(projects, [incoming[1]]), [projects[1]]);
});

test("selection and fallback explanations transition atomically and repeated acceptance is a no-op", () => {
  const initial = { selection, notice: "" };
  assert.equal(
    projectSelection(initial, { type: "accepted", snapshot: accepted }),
    initial,
  );
  assert.equal(
    projectSelection(initial, { type: "projects", projects }),
    initial,
  );
  const removed = projectSelection(initial, {
    type: "accepted",
    snapshot: { ...accepted, nodes: accepted.nodes.slice(0, 2) },
  });
  assert.equal(removed.selection.nodeId, "frame");
  assert.equal(
    removed.notice,
    "The selected layer was removed. Showing its frame.",
  );
  assert.equal(
    initial.selection.nodeId,
    "node",
    "the reducer does not mutate its input",
  );
  const unavailable = projectSelection(initial, {
    type: "projects",
    projects: [projects[1]],
  });
  assert.equal(unavailable.selection.projectId, "q");
  assert.match(unavailable.notice, /unavailable/);
  const switched = projectSelection(unavailable, {
    type: "project",
    projectId: "p",
  });
  assert.equal(switched.notice, "");
  assert.equal(switched.selection.nodeId, "");
});

test("a foreign snapshot cannot clear a current catch-up notice; validated no-op clears it", () => {
  const state = { selection, notice: "Catching up with changes…" };
  assert.equal(
    projectSelection(state, {
      type: "accepted",
      snapshot: { ...accepted, projectId: "q" },
    }),
    state,
  );
  const next = projectSelection(state, {
    type: "accepted",
    snapshot: accepted,
  });
  assert.equal(next.selection, selection);
  assert.equal(next.notice, "");
});

test("a removed page has an explicit fallback explanation", () => {
  const next = projectSelection(
    { selection: { ...selection, pageId: "removed" }, notice: "" },
    { type: "accepted", snapshot: accepted },
  );
  assert.equal(next.selection.pageId, "page");
  assert.match(next.notice, /page was removed/);
});

const twoPages = snapshot({
  nodes: [
    { id: "pg1", parentId: null, sortKey: 0, kind: "page", props: {} },
    { id: "pg2", parentId: null, sortKey: 1, kind: "page", props: {} },
    { id: "f1", parentId: "pg1", sortKey: 0, kind: "frame", props: {} },
    { id: "master", parentId: "pg1", sortKey: 1, kind: "frame", props: {} },
    { id: "label", parentId: "master", sortKey: 0, kind: "text", props: {} },
    { id: "f2", parentId: "pg2", sortKey: 0, kind: "frame", props: {} },
    { id: "t2", parentId: "f2", sortKey: 0, kind: "text", props: {} },
    {
      id: "instance",
      parentId: "f2",
      sortKey: 1,
      kind: "instance",
      props: { componentId: "master" },
    },
  ],
});
const fromLink = (query: string) =>
  projectSelection(
    { selection: readNavigation(`https://viewer.test/${query}`), notice: "" },
    { type: "accepted", snapshot: twoPages },
  );

test("partial links without page infer it from the addressed frame or node", () => {
  for (const [query, expected] of [
    ["?project=p&node=t2", ["pg2", ALL_FRAMES, "t2"]],
    ["?project=p&frame=f2", ["pg2", "f2", ""]],
    ["?project=p&frame=f2&node=t2", ["pg2", "f2", "t2"]],
    ["?project=p&node=instance%2Flabel", ["pg2", ALL_FRAMES, "instance/label"]],
    [
      "?project=p&frame=f2&node=instance%2Flabel",
      ["pg2", "f2", "instance/label"],
    ],
    ["?project=p&frame=gone&node=t2", ["pg2", ALL_FRAMES, "t2"]],
  ] as const) {
    const next = fromLink(query);
    const { pageId, frameId, nodeId } = next.selection;
    assert.deepEqual([pageId, frameId, nodeId], expected, query);
    if (query.includes("gone")) assert.match(next.notice, /frame was removed/);
    else assert.equal(next.notice, "", query);
    assert.equal(
      projectSelection(next, { type: "accepted", snapshot: twoPages }),
      next,
      "the inferred address is a fixed point",
    );
  }
});

test("partial links to truly missing targets keep first-page fallbacks and notices", () => {
  const node = fromLink("?project=p&node=gone");
  assert.deepEqual(
    [node.selection.pageId, node.selection.frameId, node.selection.nodeId],
    ["pg1", ALL_FRAMES, ""],
  );
  assert.match(node.notice, /layer was removed/);
  const frame = fromLink("?project=p&frame=gone");
  assert.deepEqual(
    [frame.selection.pageId, frame.selection.frameId],
    ["pg1", ALL_FRAMES],
  );
  assert.match(frame.notice, /frame was removed/);
  const bare = fromLink("?project=p");
  assert.deepEqual(
    [bare.selection.pageId, bare.selection.frameId, bare.notice],
    ["pg1", ALL_FRAMES, ""],
  );
});

test("an explicit page wins over the addressed frame or node's page", () => {
  const node = fromLink("?project=p&page=pg1&node=t2");
  assert.deepEqual(
    [node.selection.pageId, node.selection.frameId, node.selection.nodeId],
    ["pg1", ALL_FRAMES, ""],
  );
  assert.match(node.notice, /layer was removed/);
  const frame = fromLink("?project=p&page=pg1&frame=f2&node=t2");
  assert.deepEqual(
    [frame.selection.pageId, frame.selection.frameId],
    ["pg1", ALL_FRAMES],
  );
  assert.match(frame.notice, /frame was removed/);
  const removed = fromLink("?project=p&page=gone&frame=f2");
  assert.deepEqual(
    [removed.selection.pageId, removed.selection.frameId],
    ["pg1", ALL_FRAMES],
  );
  assert.match(removed.notice, /page was removed/);
});
