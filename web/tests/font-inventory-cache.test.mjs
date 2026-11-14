import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(new URL("../../.runtime/browsers", import.meta.url));
const { chromium } = await import("playwright");

test("font inventory caches project/revision reads and explicitly revalidates resource exceptions", async () => {
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
    const lists = [];
    let release;
    let requestStarted;
    const started = new Promise((resolve) => { requestStarted = resolve; });
    let hold = false;
    let revision = 0;
    let failed = false;
    await page.route("**/font-cache-test", (route) => route.fulfill({
      contentType: "text/html", body: '<div id="root"></div><script type="module" src="/tests/font-workflow-fixture.tsx"></script>',
    }));
    await page.route("**/api/v1/viewer", (route) => route.fulfill({ json: { readOnly: true } }));
    await page.route("**/api/v1/projects/*/fonts", async (route) => {
      const projectId = new URL(route.request().url()).pathname.split("/").at(-2);
      lists.push(projectId);
      const requestedRevision = revision;
      if (hold) await new Promise((resolve) => { release = resolve; requestStarted(); });
      await route.fulfill(failed
        ? { status: 503, json: { error: { message: "Font inventory unavailable" } } }
        : { json: { projectId, revision: requestedRevision, fonts: [{
          id: "sample", family: `${projectId} r${requestedRevision}`, weight: 400,
          style: "normal", source: "project", status: "ready",
        }] } });
    });
    await page.goto(`${vite.resolvedUrls.local[0]}font-cache-test`);
    const trigger = page.getByRole("button", { name: "Fonts", exact: true });
    const dialog = page.getByRole("dialog");
    const settle = () => page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await trigger.click();
    await dialog.getByText("p r0", { exact: true }).waitFor();
    assert.deepEqual(lists, ["p"], "StrictMode mount shares one inventory request");
    await page.keyboard.press("Escape");
    await trigger.click();
    await dialog.getByText("p r0", { exact: true }).waitFor();
    assert.deepEqual(lists, ["p"]);

    hold = true;
    await page.evaluate(() => window.selectFontProject("q"));
    await trigger.click();
    await page.waitForFunction(() => document.body.textContent.includes("Loading fonts"));
    assert.equal(await dialog.getByText("p r0", { exact: true }).count(), 0, "a different project never sees cached prior inventory");
    await started;
    hold = false;
    release();
    await dialog.getByText("q r0", { exact: true }).waitFor();
    await page.evaluate(() => window.selectFontProject("p"));
    await trigger.click();
    await dialog.getByText("p r0", { exact: true }).waitFor();
    assert.deepEqual(lists, ["p", "q"], "returning to a cached project/revision needs no read");

    revision = 1;
    await page.evaluate(() => window.selectFontRevision(1));
    await dialog.getByText("p r1", { exact: true }).waitFor();
    assert.deepEqual(lists, ["p", "q", "p"], "changed revision reads once");
    await page.evaluate(() => { window.dispatchEvent(new Event("focus")); window.dispatchEvent(new Event("online")); });
    await settle();
    assert.equal(lists.length, 3, "focus and reconnect do not refetch immutable revision data");
    await page.keyboard.press("Escape");
    revision = 2;
    await page.evaluate(() => window.selectFontRevision(2));
    await settle();
    assert.equal(lists.length, 3, "closed managers defer a new revision until opened");
    await trigger.click();
    await dialog.getByText("p r2", { exact: true }).waitFor();
    assert.equal(lists.length, 4);

    failed = true;
    await dialog.getByRole("button", { name: "Refresh list" }).click();
    await dialog.getByRole("alert").filter({ hasText: "Font inventory unavailable" }).waitFor();
    assert.equal(lists.length, 5, "explicit refresh bypasses the same-key cache");
    failed = false;
    await dialog.getByRole("button", { name: "Refresh list" }).click();
    await dialog.getByText("p r2", { exact: true }).waitFor();
    assert.equal(lists.length, 6, "failed revalidation can be retried explicitly");
    assert.deepEqual(errors, [], "failed explicit refresh produces feedback without unhandled rejection");
  } finally {
    await browser?.close();
    await vite.close();
  }
});
