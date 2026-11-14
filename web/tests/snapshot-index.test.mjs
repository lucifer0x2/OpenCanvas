import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(new URL("../../.runtime/browsers", import.meta.url));
const { chromium } = await import("playwright");
const snapshot = (projectId, revision = 1) => ({
  projectId, revision, name: projectId, schemaVersion: 1, slug: projectId,
  nodes: [
    { id: "page", parentId: null, kind: "page", sortKey: 0, props: {} },
    { id: "frame", parentId: "page", kind: "frame", sortKey: 0, props: {} },
    { id: "node", parentId: "frame", kind: "text", sortKey: 0, props: { text: `${projectId}:${revision}` } },
  ],
  tokens: [], styles: [], assets: [], fonts: [], truncated: false,
  retention: { oldestCommitRevision: 0, idempotencyPolicy: "bounded" },
});

test("accepted snapshots prime indexes before reconciliation and retain per-project identity", async () => {
  const vite = await createServer({
    root: fileURLToPath(new URL("../", import.meta.url)),
    server: { host: "127.0.0.1", port: 0 }, logLevel: "error",
  });
  let browser;
  try {
    await vite.listen();
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage();
    const errors = [];
    page.on("pageerror", (error) => errors.push(error.message));
    const responses = { a: snapshot("a"), b: snapshot("b") };
    let requests = 0;
    await page.route("**/api/v1/projects/*/snapshot", (route) => {
      const projectId = new URL(route.request().url()).pathname.split("/").at(-2);
      requests++;
      return route.fulfill({ json: responses[projectId] });
    });
    await page.route("**/snapshot-index-test", (route) => route.fulfill({
      contentType: "text/html", body: '<div id="root"></div><script type="module" src="/tests/snapshot-index-fixture.tsx"></script>',
    }));
    await page.goto(`${vite.resolvedUrls.local[0]}snapshot-index-test`);
    const probe = (method, ...args) => page.evaluate(([method, args]) => window.snapshotIndexProbe[method](...args), [method, args]);
    const wait = (projectId, revision) => page.waitForFunction(
      ([projectId, revision]) => window.snapshotIndexProbe.value().snapshot?.projectId === projectId &&
        window.snapshotIndexProbe.value().snapshot.revision === revision, [projectId, revision],
    );
    await wait("a", 1);
    const first = (await probe("value")).accepted.at(-1);
    assert.equal(first.primed, true);
    responses.a.fonts = [{ id: "new-font", family: "New metadata" }];
    await probe("refresh");
    await page.waitForFunction(() => window.snapshotIndexProbe.value().snapshot?.fonts.length === 1);
    const refetched = await probe("value");
    assert.equal(refetched.accepted.at(-1).index, first.index);
    assert.equal(refetched.snapshot.fonts[0].family, "New metadata", "equal revision still accepts new resource metadata");
    await probe("select", "b");
    await wait("b", 1);
    assert.notEqual((await probe("value")).accepted.at(-1).index, first.index);
    await probe("select", "a");
    await wait("a", 1);
    assert.equal((await probe("value")).accepted.at(-1).index, first.index);
    responses.a = snapshot("a", 2);
    await probe("refresh");
    await wait("a", 2);
    const updated = await probe("value");
    assert.notEqual(updated.accepted.at(-1).index, first.index);
    assert.equal(updated.text, "a:2");
    assert.ok(updated.accepted.every((entry) => entry.primed && entry.selected === "node"));
    assert.equal(requests, 4, "StrictMode and A→B→A reuse accepted snapshots without another GET");
    for (const projectId of ["c", "d", "e", "f", "g", "h", "i", "j"]) {
      responses[projectId] = snapshot(projectId);
      await probe("select", projectId);
      await wait(projectId, 1);
      assert.ok((await probe("value")).payloads <= 8, "only eight latest project trees are retained");
    }
    await probe("select", "a");
    await wait("a", 2);
    assert.equal(requests, 13, "evicted immutable pointers perform a fresh GET even within SWR dedupe interval");
    assert.deepEqual(errors, []);
  } finally {
    await browser?.close();
    await vite.close();
  }
});
