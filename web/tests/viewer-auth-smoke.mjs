// Chromium and the real viewer source with an isolated fixture and mocked auth policy.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { startFixture } from "./fixture.mjs";
import { startFontUi } from "./font-ui-server.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const { chromium } = await import("playwright");
const fixture = await startFixture();
let browser, server;
try {
  server = await startFontUi(fixture.baseURL);
  browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 360, height: 720 } });
  page.setDefaultTimeout(10_000);
  await page.clock.install();
  const authPath = "/api/v1/viewer/auth";
  const cookie = "viewer_test_session=valid";
  const requests = [];
  let required = true;
  let revoked = false;
  let statusUnavailable = true;
  const authorized = async (request) =>
    !required ||
    (!revoked && (await request.allHeaders()).cookie?.includes(cookie));
  page.on("request", (request) => {
    const path = new URL(request.url()).pathname;
    if (path.startsWith("/api/")) requests.push(path);
  });
  await page.route("**/api/v1/**", async (route) => {
    const request = route.request();
    const path = new URL(request.url()).pathname;
    if (path === authPath && request.method() === "GET") {
      if (statusUnavailable) return route.fulfill({ status: 503 });
      return route.fulfill({
        json: { authenticated: !!(await authorized(request)), required },
      });
    }
    if (path === authPath) {
      if (!required)
        return route.fulfill({ json: { authenticated: true, required } });
      const { password } = request.postDataJSON();
      if (password === "rate limit") return route.fulfill({ status: 429 });
      if (password === "network error") return route.abort();
      if (password !== " correct password ")
        return route.fulfill({ status: 401 });
      revoked = false;
      return route.fulfill({
        json: { authenticated: true },
        headers: {
          "Set-Cookie": `${cookie}; HttpOnly; SameSite=Strict; Path=/`,
        },
      });
    }
    if (await authorized(request)) return route.continue();
    return route.fulfill({
      status: 401,
      json: { error: { code: "UNAUTHORIZED", message: "Sign in required." } },
    });
  });

  const dialog = page.getByRole("dialog", { name: "Unlock the viewer" });
  const field = dialog.getByLabel("Password", { exact: true });
  const unlock = dialog.getByRole("button", { name: "Unlock viewer" });
  const focused = (locator) =>
    locator.evaluate((element) => element === document.activeElement);
  const viewer = `${server.base}/?project=fixture&frame=frame`;
  const preview = `${server.base}/preview?project=fixture&frame=frame`;
  const canvasReady = () =>
    page.getByText("Scene r7", { exact: true }).waitFor();
  const previewReady = () =>
    page.locator('.oc-node[data-oc-id="title"]').waitFor();
  const submit = async (password) => {
    await field.fill(password);
    await field.press("Enter");
  };
  const alert = (hasText) => page.getByRole("alert").filter({ hasText });

  await page.goto(viewer);
  await alert("Could not check viewer access.").waitFor();
  assert.ok(requests.every((path) => path === authPath));
  statusUnavailable = false;
  await page.getByRole("button", { name: "Try again" }).click();
  await dialog.waitFor();
  assert.ok(requests.every((path) => path === authPath));
  assert.equal(await field.getAttribute("autocomplete"), "current-password");
  assert.equal(await field.getAttribute("type"), "password");
  assert.equal(await focused(field), true);
  await page.keyboard.press("Shift+Tab");
  assert.equal(await focused(unlock), true);
  await page.keyboard.press("Tab");
  assert.equal(await focused(field), true);
  await page.keyboard.press("Escape");
  assert.equal(await dialog.isVisible(), true);
  const bounds = await dialog.boundingBox();
  assert.ok(bounds && bounds.x >= 0 && bounds.x + bounds.width <= 360);

  await submit("incorrect");
  await alert("Incorrect password").waitFor();
  assert.equal(await field.getAttribute("aria-invalid"), "true");
  assert.equal(await field.inputValue(), "");
  assert.equal(await focused(field), true);
  await submit("rate limit");
  await alert("Too many attempts").waitFor();
  await submit("network error");
  await alert("Could not unlock").waitFor();
  await submit(" correct password ");
  await canvasReady();
  assert.equal(await dialog.count(), 0);
  assert.equal(await page.evaluate(() => document.cookie), "");

  await page.goto(preview);
  await previewReady();
  assert.equal(await dialog.count(), 0);
  revoked = true;
  await page.clock.fastForward(31_000);
  await dialog.waitFor();
  assert.equal(await page.locator(".pv-shell").count(), 0);
  assert.equal(await page.title(), "OpenCanvas");

  requests.length = 0;
  await page.reload();
  await dialog.waitFor();
  assert.ok(requests.every((path) => path === authPath));
  await submit(" correct password ");
  await previewReady();
  revoked = true;
  await page.getByRole("button", { name: "Reload", exact: true }).click();
  await dialog.waitFor();
  assert.equal(await page.locator(".pv-shell").count(), 0);

  required = false;
  await submit("password cleared while the dialog was open");
  await previewReady();
  assert.equal(await dialog.count(), 0);
  await page.goto(viewer);
  await canvasReady();
  assert.equal(await dialog.count(), 0);
  const accessChecks = requests.filter((path) => path === authPath).length;
  await page.clock.fastForward(31_000);
  assert.equal(
    requests.filter((path) => path === authPath).length,
    accessChecks,
  );
  required = true;
  fixture.disconnect();
  await dialog.waitFor();
  assert.equal(await page.locator(".app-shell").count(), 0);
  required = false;
  await page.evaluate(() =>
    document.dispatchEvent(new Event("visibilitychange")),
  );
  await canvasReady();
  assert.equal(await dialog.count(), 0);
  console.log("Viewer auth smoke passed.");
} finally {
  await browser?.close();
  await server?.stop();
  await fixture.close();
}
