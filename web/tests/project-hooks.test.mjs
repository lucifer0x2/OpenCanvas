import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(new URL("../../.runtime/browsers", import.meta.url));
const { chromium } = await import("playwright");
const projects = [
  { projectId: "p", slug: "alpha", name: "Alpha", revision: 3 },
  { projectId: "q", slug: "beta", name: "Beta", revision: 2 },
];
const snapshot = (projectId = "p", revision = 3) => ({
  schemaVersion: 1, ...projects.find((project) => project.projectId === projectId), revision,
  nodes: [
    { id: "page", parentId: null, sortKey: 0, kind: "page", props: {} },
    { id: "frame", parentId: "page", sortKey: 0, kind: "frame", props: {} },
    { id: "node", parentId: "frame", sortKey: 0, kind: "rect", props: {} },
  ],
  tokens: [], styles: [], fonts: [], assets: [], truncated: false,
  retention: { oldestCommitRevision: 1, idempotencyPolicy: "bounded" },
});

test("project hooks preserve identity, coalesce refreshes and reject obsolete project work in StrictMode", async () => {
  const vite = await createServer({
    root: fileURLToPath(new URL("../", import.meta.url)),
    server: { host: "127.0.0.1", port: 0 }, logLevel: "error",
  });
  let browser;
  try {
    await vite.listen();
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    page.setDefaultTimeout(5000);
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    page.on("console", (message) => {
      if (message.type() === "error") errors.push(message.text());
    });
    await page.route("**/project-hooks-test", (route) => route.fulfill({
      contentType: "text/html", body: '<div id="root"></div><script type="module" src="/tests/project-hooks-fixture.tsx"></script>',
    }));
    await page.goto(`${vite.resolvedUrls.local[0]}project-hooks-test`);
    const probe = (method, ...args) => page.evaluate(([method, args]) => window.projectProbe[method](...args), [method, args]);
    const pending = async (path) => {
      await page.waitForFunction((path) => window.projectProbe?.requests().some(
        (request) => request.path === path && !request.done && !request.aborted,
      ), path);
      return (await probe("requests")).findLast(
        (request) => request.path === path && !request.done && !request.aborted,
      ).id;
    };
    const reply = async (path, body, etag, status) => probe("reply", await pending(path), body, etag, status);
    const waitRevision = (projectId, revision) => page.waitForFunction(
      ([projectId, revision]) => window.projectProbe.value().snapshot?.projectId === projectId &&
        window.projectProbe.value().snapshot?.revision === revision, [projectId, revision],
    );
    const settle = () => page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const source = async () => (await probe("sources")).findLastIndex((entry) => !entry.closed);
    const pPath = "/api/v1/projects/p/snapshot";
    const qPath = "/api/v1/projects/q/snapshot";
    const listPath = "/api/v1/projects";
    await pending(pPath);
    const pSource = await source();
    assert.ok((await probe("sources")).slice(0, pSource).every((entry) => entry.closed), "StrictMode closes its first source");
    await probe("emit", pSource, "open");
    await reply(listPath, { projects });
    await reply(pPath, snapshot(), '"p3"');
    await waitRevision("p", 3);
    await settle();
    assert.equal((await probe("requests")).filter((request) => request.path === pPath).length, 1,
      "StrictMode and first-open unchanged revision share one snapshot read");
    assert.equal((await probe("value")).selection.nodeId, "node");
    const before = await probe("value");

    await probe("emit", pSource, "project_visibility_changed");
    await reply(listPath, { projects: structuredClone(projects) });
    await reply(pPath, null, '"p3"', 304);
    await settle();
    assert.equal((await probe("value")).projectChanges, before.projectChanges, "unchanged list response preserves identity");
    assert.equal((await probe("value")).snapshotChanges, before.snapshotChanges, "304 preserves snapshot identity");
    await probe("emit", pSource, "project_activity", { projectId: "q", revision: 3 });
    await page.waitForFunction(() => window.projectProbe.value().activity.includes("q"));
    assert.equal((await probe("value")).selection.projectId, "p", "follow defaults to off");

    await probe("emit", pSource, "commit", { projectId: "p", revision: 4 });
    const fourth = await pending(pPath);
    await probe("emit", pSource, "commit", { projectId: "p", revision: 5 });
    assert.equal((await probe("requests")).filter((entry) => entry.path === pPath && !entry.done && !entry.aborted).length, 1);
    await probe("reply", fourth, snapshot("p", 4), '"p4"');
    await waitRevision("p", 4);
    await reply(pPath, snapshot("p", 5), '"p5"');
    await waitRevision("p", 5);
    await probe("emit", pSource, "commit", { projectId: "p", revision: 6 });
    const removed = snapshot("p", 6);
    removed.nodes.pop();
    await reply(pPath, removed, '"p6"');
    await waitRevision("p", 6);
    const fallback = await probe("value");
    assert.equal(fallback.selection.nodeId, "frame");
    assert.match(fallback.notice, /selected layer was removed/);

    await probe("emit", pSource, "commit", { projectId: "p", revision: 9 });
    await page.waitForFunction(() => window.projectProbe.value().notice.startsWith("Catching up"));
    await reply(pPath, snapshot("p", 9), '"p9"');
    await waitRevision("p", 9);
    await page.waitForFunction(() => window.projectProbe.value().notice === "");

    await probe("emit", pSource, "commit", { projectId: "p", revision: 10 });
    const late = await pending(pPath);
    await probe("select", "q");
    await reply(qPath, snapshot("q", 3), '"q3"');
    await waitRevision("q", 3);
    await probe("reply", late, snapshot("p", 10), '"p10"');
    await probe("follow", true);
    await settle();
    const count = (await probe("requests")).length;
    for (const name of ["open", "project_visibility_changed", "reset_required", "error"])
      await probe("emit", pSource, name);
    await probe("emit", pSource, "project_created", { projectId: "obsolete", name: "Obsolete" });
    await settle();
    assert.equal((await probe("requests")).length, count, "closed source does not refresh new project");
    assert.equal((await probe("value")).selection.projectId, "q");
    assert.equal((await probe("value")).snapshot.projectId, "q");

    // Retry keeps the accepted cursor and rejects a backwards response.
    await probe("retry");
    await reply(listPath, { projects });
    await reply(qPath, snapshot("q", 2), '"q2"');
    await settle();
    assert.equal((await probe("value")).snapshot.revision, 3);
    const qSource = await source();
    assert.match((await probe("sources"))[qSource].url, /cursor=q%3A3$/);
    await probe("emit", qSource, "error");
    await probe("emit", qSource, "error");
    await page.waitForFunction((count) => window.projectProbe.sources().length > count, qSource + 1);
    await page.waitForTimeout(800);
    assert.equal((await probe("sources")).filter((entry) => !entry.closed).length, 1, "duplicate error callbacks schedule one replacement");
    assert.equal((await probe("sources")).length, qSource + 2);

    // An authoritative list removes a hidden/deleted project and selects the first visible one.
    const reconnected = await source();
    const pReads = (await probe("requests")).filter((request) => request.path === pPath).length;
    await probe("emit", reconnected, "open");
    await reply(listPath, { projects: [projects[0]] });
    await waitRevision("p", 10);
    assert.equal((await probe("requests")).filter((request) => request.path === pPath).length, pReads,
      "returning to a cached accepted project revision needs no snapshot GET");
    assert.match((await probe("value")).notice, /project is unavailable/);
    assert.equal((await probe("value")).selection.projectId, "p");
    // Fresh events still follow when enabled; only callbacks from obsolete sources are ignored.
    await probe("emit", await source(), "project_created", projects[1]);
    await reply(listPath, { projects });
    await reply(qPath, snapshot("q", 4), '"q4"');
    await waitRevision("q", 4);
    assert.equal((await probe("value")).selection.projectId, "q");
    assert.match((await probe("value")).notice, /Following the agent/);
    await probe("unmount");
    assert.equal((await probe("sources")).filter((entry) => !entry.closed).length, 0);
    assert.deepEqual(errors, []);
  } finally {
    await browser?.close();
    await vite.close();
  }
});
