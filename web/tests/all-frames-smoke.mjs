import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { mkdir } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { PAGE_SIZES, startAllFramesFixture } from "./all-frames-fixture.mjs";
import { board, camera, checkBoard, checkServerPaths, choose, frameScope, ready, settled } from "./all-frames-browser.mjs";

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
  await ready(page, 3);
  assert.equal(await frameScope(page), "All", "fresh selection defaults to All");
  await checkBoard(page, "three");
  await choose(page, "Frame", "three / Frame 2");
  await ready(page, 1);
  assert.equal((await board(page))[0].id, "three-1");
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
  await page.getByRole("button", { name: "Zoom in", exact: true }).click();
  await page.locator(".canvas-host").focus();
  await page.keyboard.press("ArrowRight");
  await settled(page);
  const before = await camera(page);
  fixture.update();
  await ready(page, 3, 8);
  assert.deepEqual(await camera(page), before);
  assert.deepEqual(await page.evaluate(() => window.__workerSnapshots), [7, 8],
    "one structured snapshot per revision across single and All scopes");
  const fontRequests = () => fixture.requests.filter((url) =>
    url.endsWith("/fonts/font_bundled/download")).length;
  assert.equal(fontRequests(), 1, "all frames and revisions share font bytes");
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
  await settled(page);
  assert.deepEqual(await camera(page), resizedFrom, "resize preserves camera");
  await page.getByRole("button", { name: "Fit frame", exact: true }).click();
  assert.notDeepEqual(await camera(page), resizedFrom);
  await page.screenshot({ path: `${artifacts}/nine-narrow.png` });
  const boardCamera = await camera(page);
  await choose(page, "Frame", "nine / Frame 2");
  await ready(page, 1, 8);
  assert.equal((await board(page))[0].id, "nine-1");
  assert.notDeepEqual(await camera(page), boardCamera, "single-frame scope refits");
  await choose(page, "Page", "three");
  await ready(page, 3, 8);
  await checkBoard(page, "three");
  await choose(page, "Page", "nine");
  await ready(page, 9, 8);
  await checkBoard(page, "nine");
  assert.deepEqual(
    sceneRequests(),
    [],
    "healthy WASM All never calls scene APIs",
  );
  assert.equal(fontRequests(), 2, "reload owns one new shared font cache");
  const snapshots = await page.evaluate(() => window.__workerSnapshots);
  assert.deepEqual(
    snapshots,
    [8],
    "reload transfers revision once across all pages and scopes",
  );
  assert.deepEqual(errors, []);
  await context.close();
  await checkServerPaths(browser, base, fixture, artifacts);
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
