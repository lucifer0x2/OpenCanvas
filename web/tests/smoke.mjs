import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { startFixture } from "./fixture.mjs";
import {
  verifyCustomSelects,
  verifyEmptySelects,
  verifyPreviewSelect,
} from "./custom-select-smoke.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const { chromium } = await import("playwright");
const fixture = await startFixture();
const web = fileURLToPath(new URL("../", import.meta.url));
const artifacts = new URL("../../artifacts/viewer/", import.meta.url);
await mkdir(artifacts, { recursive: true });
const vite = spawn(
  process.execPath,
  [
    "node_modules/vite/bin/vite.js",
    "--host",
    "127.0.0.1",
    "--port",
    "5179",
    "--strictPort",
  ],
  {
    cwd: web,
    env: { ...process.env, OPENCANVAS_API_URL: fixture.baseURL },
    stdio: ["ignore", "pipe", "pipe"],
  },
);
let output = "";
vite.stdout.on("data", (data) => (output += data));
vite.stderr.on("data", (data) => (output += data));
let browser;
try {
  for (let attempt = 0; attempt < 50; attempt++) {
    try {
      if ((await fetch("http://127.0.0.1:5179")).ok) break;
    } catch {}
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
    deviceScaleFactor: 1,
  });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto("http://127.0.0.1:5179");
  const authored = (rev) =>
    page.getByText(`Authored r${rev}`, { exact: true }).waitFor();
  const ready = (rev) =>
    page.getByText(`Scene r${rev}`, { exact: true }).waitFor();
  await ready(7);
  await verifyCustomSelects(page);
  await verifyEmptySelects(browser, "http://127.0.0.1:5179");
  await page.getByRole("button", { name: "Headline", exact: true }).click();
  await page.getByRole("heading", { name: "Headline", exact: true }).waitFor();
  await page.screenshot({
    path: fileURLToPath(new URL("desktop.png", artifacts)),
  });
  const beforeZoom = await page.getByLabel("Zoom level").textContent();
  await page.getByRole("button", { name: "Zoom in", exact: true }).click();
  assert.notEqual(
    await page.getByLabel("Zoom level").textContent(),
    beforeZoom,
  );
  await page.getByLabel(/^Canvas\. Drag/).focus();
  await page.keyboard.press("f");
  await page.reload();
  await ready(7);
  await page.getByRole("heading", { name: "Headline", exact: true }).waitFor();

  fixture.update(8, "pending");
  await authored(8);
  await page.getByText("Scene pending", { exact: true }).waitFor();
  assert.equal(
    await page.locator("canvas").count(),
    0,
    "old scene must not be mislabeled as the current revision",
  );
  fixture.update(9);
  await ready(9);
  fixture.complete(8);
  await page.waitForTimeout(650);
  await ready(9);
  fixture.update(11);
  await ready(11); // Deliberate gap: HTTP snapshot is authoritative.
  fixture.reset();
  await ready(11);
  fixture.disconnect();
  fixture.update(12);
  await ready(12); // Reconnect cursor + snapshot recovers the commit while disconnected.
  fixture.update(13, "missing");
  await ready(13);
  await page.getByText(/Missing font: font_bundled/).waitFor();
  await page.getByText(/Missing asset: missing/).waitFor();
  await page.screenshot({
    path: fileURLToPath(new URL("missing-resources.png", artifacts)),
  });
  fixture.update(14, "failed");
  await authored(14);
  await page
    .getByRole("heading", { name: "Render failed", exact: true })
    .waitFor();
  fixture.update(15, "ready", true);
  await ready(15);
  await page.getByRole("heading", { name: "Welcome", exact: true }).waitFor();
  assert.equal(
    await page.getByRole("button", { name: "Headline", exact: true }).count(),
    0,
  );
  assert.ok(
    fixture.requests.every((request) => request.method === "GET"),
    "viewer must never mutate design data",
  );

  await page.setViewportSize({ width: 390, height: 844 });
  await page.getByRole("button", { name: "Hide layers panel" }).click();
  await page.getByRole("button", { name: "Hide inspector panel" }).click();
  await page.screenshot({
    path: fileURLToPath(new URL("narrow.png", artifacts)),
  });
  assert.equal(
    await page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
    false,
  );
  assert.deepEqual(errors, []);
  await verifyPreviewSelect(page, "http://127.0.0.1:5179");
  assert.deepEqual(errors, []);
  const evidence = {
    browser: browser.version(),
    checks: [
      "read-only GET requests",
      "selection reload",
      "zoom and keyboard fit",
      "stale job",
      "SSE gap/reset/reconnect",
      "missing font and asset",
      "render failure",
      "deleted selection fallback",
      "390px overflow",
      "custom combobox click/keyboard/typeahead/focus/outside/Tab/empty/narrow and preview",
    ],
    screenshots: ["desktop.png", "missing-resources.png", "narrow.png"],
  };
  await writeFile(
    new URL("evidence.json", artifacts),
    JSON.stringify(evidence, null, 2),
  );
  console.log(JSON.stringify(evidence, null, 2));
} catch (error) {
  console.error(output);
  throw error;
} finally {
  await browser?.close();
  vite.kill("SIGTERM");
  await fixture.close();
}
