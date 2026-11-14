import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(new URL("../../.runtime/browsers", import.meta.url));
const { chromium } = await import("playwright");
const font = { name: "Sample.ttf", mimeType: "font/ttf", buffer: Buffer.from([0, 1, 0, 0, 42]) };

test("font workflow defers unused requests, reads once, guards same-turn submits and cancels obsolete work", async () => {
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
    let capabilities = 0;
    let lists = 0;
    let publicViewer = false;
    const imports = [];
    await page.addInitScript(() => {
      window.fullFontReads = 0;
      const arrayBuffer = File.prototype.arrayBuffer;
      File.prototype.arrayBuffer = async function () {
        window.fullFontReads++;
        if (window.holdFontRead) await new Promise((resolve) => { window.releaseFontRead = resolve; });
        return arrayBuffer.call(this);
      };
    });
    await page.route("**/font-workflow-test", (route) => route.fulfill({
      contentType: "text/html", body: '<div id="root"></div><script type="module" src="/tests/font-workflow-fixture.tsx"></script>',
    }));
    await page.route("**/api/v1/viewer", (route) => {
      capabilities++;
      return route.fulfill({ json: { readOnly: publicViewer } });
    });
    await page.route("**/api/v1/projects/*/fonts", (route) => {
      if (route.request().method() === "POST") {
        imports.push(route.request().postDataJSON());
        return imports.length === 1
          ? route.fulfill({ status: 503, json: { error: { code: "UNAVAILABLE", message: "Interrupted" } } })
          : route.fulfill({ json: { committedRevision: 1 } });
      }
      lists++;
      return route.fulfill({ json: { projectId: "p", revision: 0, fonts: [] } });
    });
    await page.goto(`${vite.resolvedUrls.local[0]}font-workflow-test`);
    const trigger = page.getByRole("button", { name: "Fonts", exact: true });
    await trigger.waitFor();
    const settle = () => page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
    await settle();
    assert.equal(capabilities, 0, "closed unused manager does not request capabilities");
    assert.equal(lists, 0, "closed unused manager does not request fonts");
    assert.equal(await page.locator("form").count(), 0);
    await page.evaluate(() => window.selectFontProject("q"));
    await settle();
    assert.equal(capabilities, 0, "switching unopened projects still does no font work");
    await trigger.click();
    const dialog = page.getByRole("dialog");
    const file = dialog.getByLabel("Font file", { exact: true });
    await file.setInputFiles(font);
    await dialog.getByLabel("Font family", { exact: true }).fill("Sample");
    const add = dialog.getByRole("button", { name: "Add font", exact: true });
    await add.waitFor();
    await page.waitForFunction(() => !document.querySelector('button[type="submit"]').disabled);
    assert.equal(await page.evaluate(() => window.fullFontReads), 0, "selection reads only four header bytes");
    await page.evaluate(() => {
      window.holdFontRead = true;
      const form = document.querySelector("form");
      form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
      form.dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    });
    assert.equal(await page.evaluate(() => window.fullFontReads), 1, "two same-turn submits admit one read before React rerenders");
    await page.evaluate(() => { window.holdFontRead = false; window.releaseFontRead(); });
    await dialog.getByRole("button", { name: "Retry same import" }).waitFor();
    assert.equal(imports.length, 1);
    assert.equal(await dialog.getByLabel("Font family", { exact: true }).inputValue(), "Sample");
    await dialog.getByRole("button", { name: "Retry same import" }).click();
    await dialog.getByText(/Sample was added/).waitFor();
    await page.waitForFunction(() => [...document.querySelectorAll("button")].some((button) => button.textContent === "Refresh list" && !button.disabled));
    assert.equal(lists, 2, "successful import explicitly refreshes the same revision key");
    assert.deepEqual(imports[0], imports[1], "retry preserves exact immutable body and key");
    assert.equal(await page.evaluate(() => window.fullFontReads), 1, "retry does not reread or encode file");

    await dialog.getByLabel("Font family", { exact: true }).fill("Keep form");
    const checked = capabilities;
    const checkedLists = lists;
    await page.keyboard.press("Escape");
    assert.equal(await trigger.evaluate((element) => element === document.activeElement), true);
    await trigger.click();
    assert.equal(await dialog.getByLabel("Font family", { exact: true }).inputValue(), "Keep form");
    assert.equal(capabilities, checked, "reopen retains content and capability ownership");
    assert.equal(lists, checkedLists, "same revision reopening reuses the font inventory");
    await file.setInputFiles(font);
    await page.waitForFunction(() => !document.querySelector('button[type="submit"]').disabled);
    await page.evaluate(() => {
      window.holdFontRead = true;
      document.querySelector("form").dispatchEvent(new Event("submit", { bubbles: true, cancelable: true }));
    });
    await page.evaluate(() => window.selectFontProject("r"));
    await settle();
    await page.evaluate(() => window.releaseFontRead());
    await settle();
    assert.equal(imports.length, 2, "unmount during read cannot import into either project");
    assert.equal(await page.locator("form").count(), 0, "next unopened project has no inherited form");
    assert.equal(capabilities, checked, "capability is shared across project managers");
    publicViewer = true;
    // Listener capability cannot change on a project switch. A new page models
    // opening the public listener with its own provider/session cache.
    await page.reload();
    await trigger.click();
    await dialog.getByText(/Public viewer:/).waitFor();
    await settle();
    assert.equal(await dialog.locator("form").count(), 0, "public inventory never exposes import");
    assert.deepEqual(errors, []);
  } finally {
    await browser?.close();
    await vite.close();
  }
});
