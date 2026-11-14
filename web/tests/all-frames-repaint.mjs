// Repaint cost of the All-frames board for one-node edits, pan and memory (docs/VIEWER-FRAME-PAINT.md).
// Usage: node web/tests/all-frames-repaint.mjs [--root <checkout>] [--label after] [--edits 5] [--dpr 2]
// Builds the given checkout's viewer with test-only counters, runs a temporary native backend
// (target/debug/opencanvas-server) on a temp SQLite root and ephemeral loopback ports, and writes
// artifacts/frame-paint/<label>.json. Everything temporary is stopped and removed afterwards.
import { mkdir, readFile, rm, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { parseArgs } from "node:util";
import { EDITED, seedProject, startBackend } from "./repaint-fixture.mjs";
import { repaintProbe } from "./repaint-probe.mjs";
import { buildViewer, serveViewer } from "./repaint-viewer.mjs";

const REPO = fileURLToPath(new URL("../../", import.meta.url));
process.env.PLAYWRIGHT_BROWSERS_PATH ??= join(REPO, ".runtime/browsers");
const { values: args } = parseArgs({
  options: {
    root: { type: "string", default: REPO },
    label: { type: "string", default: "current" },
    edits: { type: "string", default: "5" },
    dpr: { type: "string", default: "2" },
    compare: { type: "string" },
  },
});
const OUT = join(REPO, "artifacts/frame-paint");
const median = (values) => [...values].sort((a, b) => a - b)[Math.floor(values.length / 2)];

async function edits(page, project, count) {
  const results = [];
  for (let i = 0; i < count; i++) {
    await page.evaluate(() => window.__repaint.reset());
    const expected = project.revision + 1;
    const seen = page.evaluate((r) => window.__repaint.whenCurrent(r), expected);
    const revision = await project.edit();
    const responded = Date.now();
    if (revision !== expected) throw new Error(`Unexpected revision ${revision}`);
    const wallMs = (await seen) - responded;
    // Late draws (the edited frame's bitmap, a zoom settle) belong to this update.
    await page.waitForTimeout(1000);
    results.push({ revision, wallMs, ...(await page.evaluate(() => window.__repaint.read())) });
  }
  return results;
}

/** Save the layer's pixels at rest; with --compare, diff against that build's capture. */
async function capture(page, name) {
  const { width, height, rgba } = await page.evaluate(() => window.__repaint.pixels());
  const bytes = Buffer.from(rgba, "base64");
  await writeFile(join(OUT, `${args.label}-${name}.rgba`), bytes);
  const result = { width, height };
  if (!args.compare) return result;
  const other = await readFile(join(OUT, `${args.compare}-${name}.rgba`)).catch(() => null);
  if (other?.length !== bytes.length) return { ...result, compared: false };
  let differing = 0, over8 = 0, maxDelta = 0;
  for (let i = 0; i < bytes.length; i += 4) {
    let delta = 0;
    for (let j = 0; j < 4; j++) delta = Math.max(delta, Math.abs(bytes[i + j] - other[i + j]));
    if (delta) differing++;
    if (delta > 8) over8++;
    maxDelta = Math.max(maxDelta, delta);
  }
  return { ...result, against: args.compare, pixels: bytes.length / 4, differing, over8, maxDelta };
}

const summary = (runs) => ({
  wallMs: median(runs.map((r) => r.wallMs)),
  paintTotal: median(runs.map((r) => r.paintTotal)),
  paintEdited: median(runs.map((r) => r.paintPerFrame[EDITED.frame] ?? 0)),
  paintMs: median(runs.map((r) => r.paintMs)),
  index: median(runs.map((r) => r.index)),
  publishes: median(runs.map((r) => r.publishes)),
  reactCommits: median(runs.map((r) => r.reactCommits)),
  konvaCommits: median(runs.map((r) => r.konvaCommits)),
  layerDraws: median(runs.map((r) => r.layerDraws.n)),
  layerDrawMs: median(runs.map((r) => r.layerDraws.total)),
  longTasks: median(runs.map((r) => r.longTasks.n)),
  longTaskMaxMs: Math.max(...runs.map((r) => r.longTasks.max)),
});

await mkdir(OUT, { recursive: true });
const build = join(OUT, `build-${args.label}`);
const backend = await startBackend();
let viewer, browser, project;
try {
  project = await seedProject(backend.base);
  const dist = await buildViewer(args.root, build);
  viewer = await serveViewer(args.root, dist, backend.base);
  const { chromium } = await import("playwright");
  browser = await chromium.launch({ headless: true });
  const context = await browser.newContext({
    viewport: { width: 1440, height: 900 },
    deviceScaleFactor: Number(args.dpr),
  });
  await context.addInitScript(repaintProbe);
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.setDefaultTimeout(180_000);
  const started = Date.now();
  const viewerAddress = new URL(viewer.url);
  viewerAddress.search = new URLSearchParams({
    project: project.projectId, page: "page", frame: ":all:",
  });
  await page.goto(viewerAddress.href);
  await page.waitForFunction(() => window.__repaint && document.querySelector(".statusbar"));
  await page.evaluate((r) => window.__repaint.whenCurrent(r), project.revision);
  const coldMs = Date.now() - started;
  await page.waitForTimeout(1500);
  const cold = await page.evaluate(() => window.__repaint.read());
  const fitScale = await page.evaluate(() => window.__repaint.scale());
  const fitEdits = await edits(page, project, Number(args.edits));
  const pixels = { fit: await capture(page, "fit") };
  const panFit = await page.evaluate(() => window.__repaint.pan(60));
  await page.evaluate((id) => window.__repaint.centerOn(id), EDITED.frame);
  const scale100 = await page.evaluate(() => window.__repaint.zoomTo(1));
  await page.waitForTimeout(1000);
  const memory100 = await page.evaluate(() => window.__repaint.read().offscreen);
  const pan100 = await page.evaluate(() => window.__repaint.pan(60));
  await page.waitForTimeout(1000);
  pixels.at100 = await capture(page, "at100");
  const edits100 = await edits(page, project, 2);
  // A second tab reads every frame from the IndexedDB scene cache: a burst of fast frames.
  const warmPage = await context.newPage();
  const warmStarted = Date.now();
  await warmPage.goto(viewerAddress.href);
  await warmPage.waitForFunction(() => window.__repaint && document.querySelector(".statusbar"));
  await warmPage.evaluate((r) => window.__repaint.whenCurrent(r), project.revision);
  const warmMs = Date.now() - warmStarted;
  await warmPage.waitForTimeout(1500);
  const warm = await warmPage.evaluate(() => window.__repaint.read());
  const report = {
    label: args.label,
    root: args.root,
    browser: browser.version(),
    dpr: Number(args.dpr),
    frames: 12,
    nodes: project.nodes,
    coldMs,
    warmMs,
    load: { cold, warm },
    fitScale,
    scale100,
    update: { fit: summary(fitEdits), at100: summary(edits100) },
    pan: { fit: panFit, at100: pan100 },
    memory: { fit: fitEdits.at(-1).offscreen, at100: memory100 },
    pixels,
    runs: { fitEdits, edits100 },
    errors,
  };
  await writeFile(join(OUT, `${args.label}.json`), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ label: report.label, coldMs, warmMs, fitScale, update: report.update,
    load: Object.fromEntries(Object.entries(report.load).map(([k, v]) => [k, { paint: v.paintTotal, index: v.index,
      publishes: v.publishes, commits: v.reactCommits, konva: v.konvaCommits, draws: v.layerDraws.n,
      longTasks: v.longTasks.n, longMax: v.longTasks.max }])),
    pan: { fit: panFit.frameIntervals, fitDraw: panFit.layerDraws, fitPaint: panFit.paintTotal,
      at100: pan100.frameIntervals, at100Draw: pan100.layerDraws, at100Paint: pan100.paintTotal },
    memory: report.memory, pixels, errors }, null, 2));
  if (errors.length) process.exitCode = 1;
} finally {
  await browser?.close();
  await viewer?.stop();
  await project?.close();
  await backend.stop();
  await rm(build, { recursive: true, force: true });
}
