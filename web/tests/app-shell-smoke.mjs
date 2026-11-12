import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import { startFixture } from "./fixture.mjs";
import { checkSidebarPreferences } from "./shell-sidebar-preferences.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const { chromium } = await import("playwright");
const fixture = await startFixture();
const vite = spawn(
  process.execPath,
  ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", "0"],
  {
    cwd: fileURLToPath(new URL("../", import.meta.url)),
    env: { ...process.env, OPENCANVAS_API_URL: fixture.baseURL },
    stdio: ["ignore", "pipe", "pipe"],
  },
);
let output = "",
  browser;
vite.stdout.on("data", (chunk) => (output += chunk));
vite.stderr.on("data", (chunk) => (output += chunk));
try {
  for (let i = 0; i < 100 && !/http:\/\/127\.0\.0\.1:\d+/.test(output); i++)
    await new Promise((resolve) => setTimeout(resolve, 50));
  const base = output.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];
  assert.ok(base, `Vite startup: ${output}`);
  browser = await chromium.launch({ headless: true });
  await checkSidebarPreferences(browser, base);
  const page = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
  });
  page.setDefaultTimeout(10_000);
  const errors = [],
    modules = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("request", (request) =>
    modules.push(new URL(request.url()).pathname),
  );
  // Count real renders without shipping a profiling/debugging API in the application.
  await page.route("**/src/components/Topbar.tsx", async (route) => {
    const response = await route.fetch();
    const original = await response.text();
    const body = original.replace(
      /function Topbar\(props\) \{/,
      "$& window.__shellTopbarRenders = (window.__shellTopbarRenders || 0) + 1;",
    );
    assert.notEqual(body, original, "Topbar render instrumentation attached");
    await route.fulfill({ response, body });
  });
  await page.addInitScript(() =>
    localStorage.setItem("is_show_notice_bar", '"false"'),
  );
  await page.goto(
    `${base}/?project=fixture&page=page&frame=frame&node=title&scene=server`,
  );
  await page
    .getByText("Scene r7", { exact: true })
    .waitFor()
    .catch(async (error) => {
      console.error({ errors, text: await page.locator("body").innerText() });
      throw error;
    });
  await page
    .getByRole("button", { name: "Show notice bar", exact: true })
    .waitFor();
  assert.ok(
    !modules.includes("/src/preview/PreviewApp.tsx"),
    "canvas route does not load preview",
  );
  await page.getByRole("button", { name: "Headline", exact: true }).click();
  await page
    .getByRole("heading", { name: "Computed bounds", exact: true })
    .waitFor();
  await page.evaluate(() => {
    window.__shellStage = window.Konva.stages[0];
    Storage.prototype.setItem = () => {
      throw new DOMException("Storage denied", "SecurityError");
    };
  });
  await page
    .getByRole("button", { name: "Show notice bar", exact: true })
    .click();
  await page
    .getByRole("button", { name: "Hide notice bar", exact: true })
    .waitFor();
  fixture.update(8, "pending");
  await page.getByText("Authored r8", { exact: true }).waitFor();
  await page.getByText("Scene pending", { exact: true }).waitFor();
  await page
    .getByText("Available when this frame's scene is ready.", { exact: true })
    .waitFor();
  const renders = await page.evaluate(() => window.__shellTopbarRenders);
  fixture.complete(8);
  await page.getByText("Scene r8", { exact: true }).waitFor();
  assert.equal(
    await page.evaluate(() => window.__shellTopbarRenders),
    renders,
    "scene completion does not rerender topbar",
  );
  assert.equal(
    await page.evaluate(() => window.Konva.stages[0] === window.__shellStage),
    true,
    "same-scope stage remains mounted",
  );
  assert.deepEqual(errors, []);
  await page.close();
  console.log(
    "ok - malformed boolean defaults, denied storage toggles, stale inspector hides, stable camera and isolated chrome",
  );

  for (const stored of ["{invalid", "null", "0", "{}", "[]"]) {
    const preference = await browser.newPage();
    await preference.addInitScript(
      (value) => localStorage.setItem("is_show_notice_bar", value),
      stored,
    );
    await preference.goto(`${base}/?scene=server`);
    await preference
      .getByRole("button", { name: "Show notice bar", exact: true })
      .waitFor();
    await preference.close();
  }
  const denied = await browser.newPage();
  await denied.addInitScript(() => {
    Storage.prototype.getItem = () => {
      throw new DOMException("Storage denied", "SecurityError");
    };
    Storage.prototype.setItem = () => {
      throw new DOMException("Storage denied", "SecurityError");
    };
  });
  await denied.goto(`${base}/?scene=server`);
  await denied
    .getByRole("button", { name: "Show notice bar", exact: true })
    .click();
  await denied
    .getByRole("button", { name: "Hide notice bar", exact: true })
    .waitFor();
  await denied.close();
  console.log(
    "ok - malformed JSON/non-boolean preferences and denied storage reads remain usable",
  );

  const preview = await browser.newPage();
  const previewModules = [];
  preview.on("request", (request) =>
    previewModules.push(new URL(request.url()).pathname),
  );
  await preview.goto(
    `${base}/preview?project=fixture&frame=frame&scene=server`,
  );
  await preview.getByText("Motion preview", { exact: true }).waitFor();
  assert.ok(
    !previewModules.includes("/src/App.tsx"),
    "preview does not load canvas app",
  );
  await preview.close();

  for (const path of ["/", "/preview?project=fixture&frame=frame"]) {
    const locked = await browser.newPage();
    const requests = [];
    locked.on("request", (request) =>
      requests.push(new URL(request.url()).pathname),
    );
    await locked.route("**/api/v1/viewer/auth", (route) =>
      route.fulfill({ json: { authenticated: false, required: true } }),
    );
    await locked.goto(`${base}${path}`);
    await locked.getByRole("dialog").waitFor();
    assert.ok(
      !requests.includes("/src/App.tsx") &&
        !requests.includes("/src/preview/PreviewApp.tsx"),
      "locked viewer does not load either route",
    );
    assert.ok(
      !requests.some((url) => url.startsWith("/api/v1/projects")),
      "locked viewer never requests project data",
    );
    await locked.close();
  }
  console.log("ok - canvas/preview route split and authentication gate");

  const failed = await browser.newPage();
  let releaseModule;
  const heldModule = new Promise((resolve) => {
    releaseModule = resolve;
  });
  await failed.route("**/src/App.tsx", async (route) => {
    await heldModule;
    await route.abort("failed");
  });
  await failed.goto(base);
  await failed
    .getByRole("status")
    .filter({ hasText: "Loading viewer…" })
    .waitFor();
  releaseModule();
  await failed
    .getByRole("alert")
    .filter({ hasText: "Could not load this viewer." })
    .waitFor();
  await failed.unroute("**/src/App.tsx");
  await failed.getByRole("button", { name: "Try again", exact: true }).click();
  await failed
    .getByRole("combobox", { name: "Project", exact: true })
    .waitFor();
  await failed.close();
  console.log(
    "ok - route chunk has accessible pending/error status and retry recovers",
  );
} finally {
  await browser?.close();
  const exited = once(vite, "exit");
  vite.kill("SIGTERM");
  await exited;
  await fixture.close();
}
