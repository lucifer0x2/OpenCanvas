import assert from "node:assert/strict";
import test from "node:test";
import { fileURLToPath } from "node:url";
import { createServer } from "vite";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(new URL("../../.runtime/browsers", import.meta.url));
const { chromium } = await import("playwright");

test("viewer capability shares its pending read and fails closed per provider", async () => {
  const vite = await createServer({
    root: fileURLToPath(new URL("../", import.meta.url)),
    server: { host: "127.0.0.1", port: 0 }, logLevel: "error",
  });
  let browser;
  try {
    await vite.listen();
    browser = await chromium.launch({ headless: true });
    for (const mode of ["writable", "read only", "failure"]) {
      const page = await browser.newPage();
      page.setDefaultTimeout(5000);
      let requests = 0;
      let release;
      let requestStarted;
      const started = new Promise((resolve) => { requestStarted = resolve; });
      await page.route("**/viewer-access-test", (route) => route.fulfill({
        contentType: "text/html", body: '<div id="root"></div><script type="module" src="/tests/viewer-access-fixture.tsx"></script>',
      }));
      await page.route("**/api/v1/viewer", async (route) => {
        requests++;
        await new Promise((resolve) => { release = resolve; requestStarted(); });
        await route.fulfill(mode === "failure"
          ? { status: 503, json: { error: { message: "Unavailable" } } }
          : { json: { readOnly: mode !== "writable" } });
      });
      await page.goto(`${vite.resolvedUrls.local[0]}viewer-access-test`);
      const first = page.getByLabel("first");
      await first.getByText("pending", { exact: true }).waitFor();
      await page.getByLabel("second").getByText("pending", { exact: true }).waitFor();
      await started;
      assert.equal(requests, 1, "concurrent StrictMode consumers share the capability read");
      release();
      const expected = mode === "writable" ? "writable" : "read only";
      await first.getByText(expected, { exact: true }).waitFor();
      await page.getByLabel("second").getByText(expected, { exact: true }).waitFor();
      assert.equal(requests, 1);
      await page.close();
    }
  } finally {
    await browser?.close();
    await vite.close();
  }
});
