import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { PAGE_SIZES, startAllFramesFixture } from "./all-frames-fixture.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const fixture = await startAllFramesFixture();
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
vite.stdout.on("data", (value) => {
  output += value;
});
vite.stderr.on("data", (value) => {
  output += value;
});
const artifacts = fileURLToPath(
  new URL("../../artifacts/all-frames/", import.meta.url),
);
await mkdir(artifacts, { recursive: true });
const sceneRequests = () =>
  fixture.requests.filter((url) => /\/scenes?\?/.test(url));
const choose = async (page, label, option) => {
  await page.getByRole("combobox", { name: label, exact: true }).click();
  await page.getByRole("option", { name: option, exact: true }).click();
};
const board = (page) =>
  page.evaluate(() => {
    const stage = window.Konva.stages.find(
      (entry) => entry.container()?.isConnected,
    );
    if (!stage) return [];
    return stage.find(".canvas-frame").map((frame) => ({
      id: frame.id(),
      x: frame.x(),
      y: frame.y(),
      width: frame.findOne("Rect").width(),
      height: frame.findOne("Rect").height(),
      label: frame.findOne("Text")?.text(),
    }));
  });
const camera = (page) =>
  page.evaluate(() => {
    const stage = window.Konva.stages.find(
      (entry) => entry.container()?.isConnected,
    );
    return { x: stage.x(), y: stage.y(), scale: stage.scaleX() };
  });
async function ready(page, count, revision = 7) {
  await page
    .getByText(count ? `Scene r${revision}` : "No scene", { exact: true })
    .waitFor();
  assert.equal((await board(page)).length, count);
}
async function checkBoard(page, pageId) {
  const entries = await board(page);
  assert.deepEqual(
    entries.map((entry) => entry.id),
    PAGE_SIZES[pageId].map((_, i) => `${pageId}-${i}`),
  );
  for (const entry of entries) {
    assert.ok(entry.label.startsWith(pageId));
    assert.ok(entry.y >= 32);
    for (const other of entries) {
      if (entry.id === other.id) continue;
      assert.ok(
        entry.x + entry.width + 63 <= other.x ||
          other.x + other.width + 63 <= entry.x ||
          entry.y + entry.height + 95 <= other.y ||
          other.y + other.height + 95 <= entry.y,
      );
    }
  }
}
try {
  for (let i = 0; i < 100 && !output.match(/http:\/\/127\.0\.0\.1:\d+/); i++)
    await new Promise((resolve) => setTimeout(resolve, 50));
  const base = output.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];
  assert.ok(base, output);
  const { chromium } = await import("playwright");
  browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1600, height: 1000 },
    permissions: ["clipboard-read", "clipboard-write"],
  });
  await context.addInitScript(() => {
    window.__workerSnapshots = [];
    const OriginalWorker = Worker;
    window.Worker = class extends OriginalWorker {
      postMessage(message, ...rest) {
        if (message.snapshot)
          window.__workerSnapshots.push(message.snapshot.revision);
        return super.postMessage(message, ...rest);
      }
    };
  });
  const page = await context.newPage(),
    errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(base);
  await ready(page, 1);
  await choose(page, "Frame", "All");
  await ready(page, 3);
  await checkBoard(page, "three");
  assert.equal(new Set((await board(page)).map((entry) => entry.y)).size, 1);
  await page.screenshot({ path: `${artifacts}/three.png` });
  // Hit selection is board-translated while inspector and copied links remain frame-local.
  const target = (await board(page))[1],
    cam = await camera(page);
  const pane = await page.locator(".canvas-host").boundingBox();
  await page.mouse.click(
    pane.x + cam.x + (target.x + 40) * cam.scale,
    pane.y + cam.y + (target.y + 60) * cam.scale,
  );
  await page.getByRole("heading", { name: "Accent 2", exact: true }).waitFor();
  assert.equal(
    await page
      .getByRole("combobox", { name: "Frame", exact: true })
      .innerText(),
    "All",
  );
  await page.getByRole("button", { name: "Copy link", exact: true }).click();
  const link = await page.evaluate(() => navigator.clipboard.readText());
  assert.equal(new URL(link).searchParams.get("frame"), "three-1");
  assert.equal(new URL(link).searchParams.get("node"), "three-1-shape");
  assert.match(await page.locator(".bounds-grid").innerText(), /x\s+24/);
  const before = await camera(page);
  fixture.update();
  await ready(page, 3, 8);
  assert.deepEqual(await camera(page), before);
  await page.reload();
  await ready(page, 3, 8);
  assert.equal(
    await page
      .getByRole("combobox", { name: "Frame", exact: true })
      .innerText(),
    "All",
  );
  for (const id of ["four", "nine", "mixed", "one", "zero"]) {
    await choose(page, "Page", id);
    await ready(page, PAGE_SIZES[id].length, 8);
    await checkBoard(page, id);
    await page.screenshot({ path: `${artifacts}/${id}.png` });
  }
  await page.getByRole("heading", { name: "No frames on this page" }).waitFor();
  await choose(page, "Page", "nine");
  await ready(page, 9, 8);
  const resizedFrom = await camera(page);
  await page.setViewportSize({ width: 1100, height: 800 });
  await page.waitForTimeout(100);
  assert.deepEqual(await camera(page), resizedFrom, "resize preserves camera");
  await page.getByRole("button", { name: "Fit frame", exact: true }).click();
  assert.notDeepEqual(await camera(page), resizedFrom);
  await page.screenshot({ path: `${artifacts}/nine-narrow.png` });
  await choose(page, "Frame", "nine / Frame 2");
  await ready(page, 1, 8);
  await choose(page, "Frame", "All");
  await ready(page, 9, 8);
  assert.deepEqual(
    sceneRequests(),
    [],
    "healthy WASM All never calls scene APIs",
  );
  assert.equal(
    fixture.requests.filter((url) =>
      url.endsWith("/fonts/font_bundled/download"),
    ).length,
    1,
  );
  const snapshots = await page.evaluate(() => window.__workerSnapshots);
  assert.deepEqual(
    snapshots,
    [7, 8],
    "one structured snapshot per accepted revision across all frames/scopes",
  );
  assert.deepEqual(errors, []);
  await context.close();
  // Server batches never queue more than four, and failures stay visible in their slots.
  const server = await browser.newContext({
    viewport: { width: 1440, height: 1000 },
  });
  const serverPage = await server.newPage();
  await serverPage.goto(`${base}/?scene=server`);
  await ready(serverPage, 1, 8);
  await choose(serverPage, "Page", "nine");
  await ready(serverPage, 1, 8);
  fixture.fail("nine-4");
  await choose(serverPage, "Frame", "All");
  await serverPage
    .getByText("Render failed", { exact: true })
    .first()
    .waitFor();
  assert.equal((await board(serverPage)).length, 9);
  await serverPage.screenshot({ path: `${artifacts}/partial-failure.png` });
  fixture.fail("");
  await serverPage.getByRole("button", { name: "Retry render" }).click();
  await ready(serverPage, 9, 8);
  assert.deepEqual(fixture.metrics(), { maxActive: 4, overlap: false });
  await server.close();
  // A missing WASM module uses the same bounded server page contract with visible reason.
  const fallback = await browser.newContext();
  await fallback.route("**/scene-wasm/opencanvas_scene_wasm.js", (route) =>
    route.fulfill({ status: 404, body: "unavailable" }),
  );
  const fallbackPage = await fallback.newPage();
  await fallbackPage.goto(base);
  await ready(fallbackPage, 1, 8);
  await choose(fallbackPage, "Frame", "All");
  await ready(fallbackPage, 3, 8);
  await fallbackPage
    .getByText(/Browser scene unavailable; using server scene/)
    .waitFor();
  assert.deepEqual(fixture.metrics(), { maxActive: 4, overlap: false });
  await fallback.close();
  console.log(
    "All frames smoke passed: 0/1/3/4/9/mixed, labels/spacing, selection/link/bounds, SSE/reload, scope/resize/Fit, shared WASM/font caches, bounded server batches, partial failure/retry and WASM fallback.",
  );
} finally {
  await browser?.close();
  const exited = once(vite, "exit");
  vite.kill("SIGTERM");
  await exited;
  await fixture.close();
}
