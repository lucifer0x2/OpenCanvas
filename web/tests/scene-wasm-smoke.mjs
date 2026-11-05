import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import { startSceneFixture } from "./scene-wasm-fixture.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const fixture = await startSceneFixture();
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
vite.stdout.on("data", (value) => (output += value));
vite.stderr.on("data", (value) => (output += value));
const sceneRequests = () =>
  fixture.requests.filter((url) => url.includes("/scene?"));
const snapshotRequests = () =>
  fixture.requests.filter((url) => url.endsWith("/snapshot"));
const fontRequests = () =>
  fixture.requests.filter((url) =>
    url.endsWith("/fonts/font_bundled/download"),
  );

try {
  for (let i = 0; i < 100 && !output.match(/http:\/\/127\.0\.0\.1:\d+/); i++)
    await new Promise((resolve) => setTimeout(resolve, 50));
  const base = output.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];
  assert.ok(base, `Vite startup: ${output}`);
  const { chromium } = await import("playwright");
  browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 1000 },
  });
  await context.addInitScript(() => {
    window.__sceneMessages = [];
    window.__sceneResults = [];
    const NativeWorker = window.Worker;
    window.Worker = class extends NativeWorker {
      constructor(...args) {
        super(...args);
        this.addEventListener("message", ({ data }) => {
          if (data.result?.provenance) window.__sceneResults.push(data.result);
        });
      }
      postMessage(message, ...rest) {
        window.__sceneMessages.push({
          kind: message.kind,
          revision: message.request.revision,
          accepted: message.snapshot?.revision,
        });
        return super.postMessage(message, ...rest);
      }
    };
  });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(`${base}/?scene=wasm`);
  await page.getByText("Scene r7", { exact: true }).waitFor();
  assert.deepEqual(
    sceneRequests(),
    [],
    "successful WASM frame makes no /scene call",
  );
  assert.equal(
    fontRequests().length,
    1,
    "WASM and FontFace share the same validated bytes",
  );
  assert.equal(
    fixture.requests.some((url) => url.includes("unused_missing/download")),
    false,
  );
  assert.equal(
    await page.evaluate(
      () =>
        [...document.fonts].filter((font) => font.status === "loaded").length,
    ),
    1,
  );
  await page.waitForTimeout(200);
  const before = snapshotRequests().length;
  fixture.holdSnapshots(true);
  fixture.update(8);
  for (let i = 0; i < 100 && snapshotRequests().length === before; i++)
    await page.waitForTimeout(10);
  assert.equal(
    snapshotRequests().length,
    before + 1,
    "SSE uses the existing authoritative refresh only",
  );
  assert.equal(
    await page.evaluate(() =>
      window.__sceneMessages.some((message) => message.revision === 8),
    ),
    false,
    "SSE hint must not compute with an old snapshot",
  );
  fixture.holdSnapshots(false);
  await page.getByText("Scene r8", { exact: true }).waitFor();
  const evidence = await page.evaluate(() => ({
    messages: window.__sceneMessages,
    results: window.__sceneResults,
  }));
  assert.ok(
    evidence.messages.every(
      (message) =>
        message.accepted === undefined || message.revision === message.accepted,
    ),
  );
  for (const revision of [7, 8]) {
    const messages = evidence.messages.filter(
      (message) => message.revision === revision,
    );
    assert.equal(
      messages.filter((message) => message.accepted !== undefined).length,
      1,
      "one structured snapshot per revision; prepare/compute otherwise reference the cache",
    );
    assert.ok(
      messages.some(
        (message) =>
          message.kind === "compute" && message.accepted === undefined,
      ),
    );
  }
  assert.ok(
    evidence.results
      .find((scene) => scene.provenance.revision === 8)
      .nodes.some((node) =>
        node.lines.some((line) =>
          line.text.includes("Accepted snapshot revision 8"),
        ),
      ),
  );
  assert.equal(fontRequests().length, 1, "SSE revision reuses font bytes");
  assert.deepEqual(sceneRequests(), []);
  assert.deepEqual(errors, []);
  await context.close();

  const fallback = await browser.newContext();
  await fallback.route("**/scene-wasm/opencanvas_scene_wasm.js", (route) =>
    route.fulfill({ status: 404, body: "missing module" }),
  );
  const failed = await fallback.newPage();
  await failed.goto(`${base}/?scene=wasm`);
  await failed.getByText("Scene r8", { exact: true }).waitFor();
  await failed
    .getByText(/Browser scene unavailable; using server scene/)
    .waitFor();
  assert.equal(
    sceneRequests().length,
    1,
    "module failure calls the original scene route",
  );
  await fallback.close();

  fixture.corruptFont(true);
  const damaged = await browser.newContext();
  const damagedPage = await damaged.newPage();
  const beforeDamaged = sceneRequests().length;
  const beforeFonts = fontRequests().length;
  await damagedPage.goto(`${base}/?scene=wasm`);
  await damagedPage.getByText("Scene r8", { exact: true }).waitFor();
  await damagedPage
    .getByText(/Browser scene unavailable; using server scene:.*hash mismatch/)
    .waitFor();
  assert.equal(
    sceneRequests().length,
    beforeDamaged + 1,
    "invalid SHA falls back without using corrupt font bytes",
  );
  fixture.corruptFont(false);
  fixture.update(9);
  await damagedPage.getByText("Scene r9", { exact: true }).waitFor();
  assert.equal(
    await damagedPage
      .getByText(/Browser scene unavailable; using server scene/)
      .count(),
    0,
  );
  assert.equal(
    sceneRequests().length,
    beforeDamaged + 1,
    "a later revision retries browser compute after a failed font load",
  );
  assert.equal(
    fontRequests().length,
    beforeFonts + 2,
    "failed bytes are evicted and successful retry is shared with FontFace",
  );
  await damaged.close();

  const ordinary = await browser.newContext();
  const normal = await ordinary.newPage();
  const wasmLoads = [];
  normal.on("request", (req) => {
    if (req.url().includes("scene-wasm/") || req.url().includes("scene.worker"))
      wasmLoads.push(req.url());
  });
  const beforeOrdinary = sceneRequests().length;
  await normal.goto(base);
  await normal.getByText("Scene r9", { exact: true }).waitFor();
  assert.equal(sceneRequests().length, beforeOrdinary + 1);
  assert.deepEqual(
    wasmLoads,
    [],
    "default route never starts or initializes WASM",
  );
  await ordinary.close();
  console.log(
    "WASM scene smoke passed: frame compute, shared font bytes, unused missing face, accepted SSE snapshot, module/hash fallback and retry, default lazy path.",
  );
} finally {
  await browser?.close();
  const exited = once(vite, "exit");
  vite.kill("SIGTERM");
  await exited;
  await fixture.close();
}
