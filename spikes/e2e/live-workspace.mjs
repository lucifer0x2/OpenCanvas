// Live workspace group (Task08): new projects and other projects' commits reach an open viewer
// without reload, over at most one EventSource per tab, including project switches and reconnect.
import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { connectMcp, waitFor } from "./protocol.mjs";
import { startServer } from "./server.mjs";
import { assertReadOnly } from "./browser.mjs";
import { openTab } from "./live-tab.mjs";

const page = (name) => [
  {
    op: "create_node",
    id: "page",
    kind: "page",
    parentId: null,
    props: { name },
  },
];

export async function verifyLiveWorkspace(browser) {
  const dataRoot = await mkdtemp(join(tmpdir(), "opencanvas-live-workspace-"));
  let server, mcp, view;
  const latency = {};
  const timed = async (key, action, until) => {
    const started = Date.now();
    const result = await action();
    await until(result);
    latency[key] = Date.now() - started;
    return result;
  };
  try {
    server = await startServer(dataRoot);
    mcp = await connectMcp(server.base);
    view = await openTab(browser, server.base);
    const { page: tab, streams } = view;
    const picker = tab.getByLabel("Project", { exact: true });
    const selected = (id) =>
      waitFor(async () => (await picker.inputValue()) === id, id, 5_000);
    const streamOn = (path) =>
      waitFor(async () => streams.current().join() === path, path, 5_000);
    await tab.getByText("No project selected", { exact: true }).waitFor();
    await streamOn("/api/v1/events");

    // Empty workspace: the first created project is selected and its stream replaces the workspace one.
    const a = await timed(
      "createFirstToSelected",
      () =>
        mcp.call("create_project", { slug: "live-alpha", name: "Live alpha" }),
      (created) => selected(created.projectId),
    );
    await streamOn(`/api/v1/projects/${a.projectId}/events`);
    await mcp.call("apply_batch", {
      projectId: a.projectId,
      expectedRevision: 0,
      idempotencyKey: "alpha-1",
      operations: page("Alpha"),
    });
    await tab.getByText("Authored r1", { exact: true }).waitFor();

    // Another project appears in the selector while A stays in view.
    const b = await timed(
      "createOtherToOption",
      () =>
        mcp.call("create_project", { slug: "live-beta", name: "Live beta" }),
      (created) =>
        picker
          .locator(`option[value="${created.projectId}"]`)
          .waitFor({ state: "attached" }),
    );
    assert.ok(
      latency.createOtherToOption < 2_000,
      `${latency.createOtherToOption} ms`,
    );
    const snapshotsOfA = () =>
      view.requests.filter(
        (r) => r.path === `/api/v1/projects/${a.projectId}/snapshot`,
      ).length;
    const before = snapshotsOfA();
    await timed(
      "commitOtherToMark",
      () =>
        mcp.call("apply_batch", {
          projectId: b.projectId,
          expectedRevision: 0,
          idempotencyKey: "beta-1",
          operations: page("Beta"),
        }),
      () =>
        picker
          .locator("option", { hasText: "● Live beta" })
          .waitFor({ state: "attached" }),
    );
    await tab
      .getByText("Live beta has new changes", { exact: false })
      .waitFor();
    await tab.waitForTimeout(500);
    assert.equal(snapshotsOfA(), before, "a commit to B must not refetch A");
    assert.equal(await picker.inputValue(), a.projectId);

    // Selecting B shows its newest revision and clears its mark.
    await picker.selectOption(b.projectId);
    await tab.getByText("Authored r1", { exact: true }).waitFor();
    await streamOn(`/api/v1/projects/${b.projectId}/events`);
    assert.equal(await picker.locator("option", { hasText: "●" }).count(), 0);

    // Reconnect after a backend restart keeps a single stream and refetches the list.
    const port = server.port;
    const opened = streams.total;
    const lists = view.requests.filter(
      (r) => r.path === "/api/v1/projects",
    ).length;
    await mcp.close();
    await server.stop();
    server = await startServer(dataRoot, port);
    mcp = await connectMcp(server.base);
    const bPath = `/api/v1/projects/${b.projectId}/events`;
    await waitFor(
      async () => streams.total > opened && streams.current().join() === bPath,
      "reconnected stream",
      15_000,
    );
    await waitFor(
      async () =>
        view.requests.filter((r) => r.path === "/api/v1/projects").length >
        lists,
      "list refetch on reopen",
      5_000,
    );

    // Follow agent (default off, persisted) moves the viewer to the project the agent touches.
    const toggle = tab.getByLabel("Follow agent");
    assert.equal(
      await toggle.isChecked(),
      false,
      "follow agent defaults to off",
    );
    await toggle.check();
    const c = await timed(
      "followCreateToSelected",
      () =>
        mcp.call("create_project", { slug: "live-gamma", name: "Live gamma" }),
      (created) => selected(created.projectId),
    );
    await streamOn(`/api/v1/projects/${c.projectId}/events`);
    await timed(
      "followCommitToSelected",
      () =>
        mcp.call("apply_batch", {
          projectId: a.projectId,
          expectedRevision: 1,
          idempotencyKey: "alpha-2",
          operations: [
            { op: "update_node", id: "page", props: { name: "Alpha 2" } },
          ],
        }),
      () => selected(a.projectId),
    );
    await tab.getByText("Authored r2", { exact: true }).waitFor();
    assert.equal(
      await tab.evaluate(() => localStorage.getItem("opencanvas.followAgent")),
      "true",
    );

    assert.equal(
      streams.max,
      1,
      `EventSource paths: ${streams.paths.join(", ")}`,
    );
    assertReadOnly(view);
    const events = await tab.evaluate(() => globalThis.__sse);
    return {
      check:
        "live workspace: project_created/project_activity, one EventSource per tab, follow agent",
      eventSources: {
        maxOpenPerTab: streams.max,
        opened: streams.total,
        paths: streams.paths,
      },
      latencyMs: latency,
      events: events.map(({ path, type, data }) => ({ path, type, data })),
    };
  } finally {
    await Promise.allSettled([view?.page.close(), mcp?.close()]);
    await server?.stop();
    await rm(dataRoot, { recursive: true, force: true });
  }
}
