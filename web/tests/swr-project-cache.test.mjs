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
const snapshot = (projectId, revision) => ({
  ...projects.find((project) => project.projectId === projectId), revision,
  schemaVersion: 1, nodes: [], tokens: [], styles: [], assets: [], fonts: [], truncated: false,
  retention: { oldestCommitRevision: 0, idempotencyPolicy: "bounded" },
});

test("SWR canonical revisions, authoritative pagination, explicit invalidation and session isolation", async () => {
  const vite = await createServer({ root: fileURLToPath(new URL("../", import.meta.url)),
    server: { host: "127.0.0.1", port: 0 }, logLevel: "error" });
  let browser;
  try {
    await vite.listen();
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    page.setDefaultTimeout(5000);
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    await page.route("**/swr-cache-test", (route) => route.fulfill({ contentType: "text/html",
      body: '<div id="root"></div><script type="module" src="/tests/project-hooks-fixture.tsx"></script>' }));
    await page.goto(`${vite.resolvedUrls.local[0]}swr-cache-test`);
    const probe = (method, ...args) => page.evaluate(([method, args]) => window.projectProbe[method](...args), [method, args]);
    const pending = async (path) => {
      await page.waitForFunction((path) => window.projectProbe?.requests().some(
        (request) => request.path === path && !request.done && !request.aborted), path);
      return (await probe("requests")).findLast((request) => request.path === path && !request.done && !request.aborted);
    };
    const reply = async (path, body, etag, status) => probe("reply", (await pending(path)).id, body, etag, status);
    const wait = (projectId, revision) => page.waitForFunction(([projectId, revision]) =>
      window.projectProbe.value().snapshot?.projectId === projectId && window.projectProbe.value().snapshot?.revision === revision,
    [projectId, revision]);
    const settle = () => page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    const source = async () => (await probe("sources")).findLastIndex((entry) => !entry.closed);
    const count = async (path) => (await probe("requests")).filter((request) => request.path === path).length;
    const p = "/api/v1/projects/p/snapshot", q = "/api/v1/projects/q/snapshot", list = "/api/v1/projects";

    await reply(p, snapshot("p", 7), '"p7"'); // Latest can advance beyond the initial list watermark.
    await reply(list, { projects: [projects[0]], nextCursor: "second" });
    await reply(`${list}?cursor=second`, { projects: [projects[1]] });
    await wait("p", 7);
    await settle();
    assert.equal(await count(p), 1, "StrictMode and initial watermark convergence issue one GET");
    assert.deepEqual((await probe("cache")).map(({ projectId, revision }) => ({ projectId, revision })),
      [{ projectId: "p", revision: 7 }], "newer payload is cached only under its actual canonical revision");
    const active = await source();
    await probe("emit", active, "open");
    await reply(list, { projects: [{ ...projects[0], revision: 7 }, projects[1]] });
    await probe("emit", active, "commit", { projectId: "p", revision: 7 });
    await probe("emit", active, "commit", { projectId: "q", revision: 99 });
    await probe("emit", active, "project_activity", { projectId: "ghost", revision: 1 });
    await settle();
    assert.equal(await count(p), 1, "same revision/list refresh and foreign commit do not refetch");
    assert.deepEqual((await probe("value")).projects.map((project) => project.projectId), ["p", "q"],
      "workspace hints cannot add list membership");
    await probe("select", "q");
    await reply(q, snapshot("q", 2), '"q2"');
    await wait("q", 2);
    await probe("select", "p");
    await wait("p", 7);
    assert.equal(await count(p), 1, "A→B→A uses its accepted canonical snapshot");

    const changes = (await probe("value")).snapshotChanges;
    await probe("retry");
    await reply(list, { projects });
    assert.equal((await pending(p)).validator, '"p7"');
    await reply(p, null, '"p7"', 304);
    await settle();
    assert.equal((await probe("value")).snapshotChanges, changes, "explicit 304 preserves object identity");

    await probe("emit", await source(), "commit", { projectId: "p", revision: 8 });
    const obsolete = await pending(p);
    await probe("retry");
    await reply(list, { projects });
    const fresh = await pending(p);
    assert.notEqual(fresh.id, obsolete.id, "manual retry replaces a hung obsolete epoch");
    assert.equal((await probe("requests")).find((request) => request.id === obsolete.id).aborted, true);
    await probe("reply", fresh.id, snapshot("q", 99), '"foreign"');
    await settle();
    assert.equal((await probe("value")).snapshot.projectId, "p", "foreign payload cannot publish");
    assert.equal((await probe("value")).snapshot.revision, 7);
    await probe("retry");
    await reply(list, { projects });
    await reply(p, snapshot("p", 8), '"p8"');
    await wait("p", 8);

    await probe("emit", await source(), "project_visibility_changed");
    await reply(p, { error: { message: "Project unavailable", code: "NOT_FOUND" } }, undefined, 404);
    await page.waitForFunction(() => window.projectProbe.value().snapshot === null);
    await reply(list, { projects: [projects[1]] });
    await reply(q, snapshot("q", 2), '"q2-new-visibility"');
    await wait("q", 2);
    assert.equal((await probe("cache")).some((entry) => entry.projectId === "p"), false,
      "404/list removal evict inaccessible payloads");

    // Refresh errors remain visible through SWR without an unhandled Promise rejection.
    await probe("retry");
    await reply(list, { error: { code: "UNAVAILABLE", message: "List temporarily unavailable" } }, undefined, 503);
    await reply(q, null, '"q2-new-visibility"', 304);
    await page.waitForFunction(() => window.projectProbe.value().error === "List temporarily unavailable");
    await probe("retry");
    await reply(list, { projects: [projects[1]] });
    await reply(q, null, '"q2-new-visibility"', 304);
    await page.waitForFunction(() => window.projectProbe.value().error === "");

    await probe("emit", await source(), "commit", { projectId: "q", revision: 3 });
    const oldSession = await pending(q);
    await probe("authorize", false);
    await page.getByText("Locked", { exact: true }).waitFor();
    assert.equal((await probe("requests")).find((request) => request.id === oldSession.id).aborted, true);
    await probe("reply", oldSession.id, snapshot("q", 3), '"old-session"');
    await probe("authorize", true);
    const newSession = await pending(q);
    assert.equal(newSession.validator, null, "a new authorized lifetime has no previous session validator");
    await reply(list, { projects: [] });
    await page.waitForFunction(() => window.projectProbe.value().selection.projectId === "");
    assert.equal((await probe("requests")).find((request) => request.id === newSession.id).aborted, true,
      "authoritative removal cancels even a project's first pending snapshot");
    await probe("reply", newSession.id, snapshot("q", 2), '"q2-new-session"');
    await settle();
    assert.deepEqual(await probe("cache"), [], "a late removed-project response cannot repopulate the cache");
    await probe("emit", await source(), "project_created", projects[1]);
    await reply(list, { projects: [projects[1]] });
    await reply(q, snapshot("q", 2), '"q2-new-session"');
    await wait("q", 2);
    assert.deepEqual(errors, []);
    await probe("unmount");
  } finally { await browser?.close(); await vite.close(); }
});
