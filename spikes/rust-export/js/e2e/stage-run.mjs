// Part D, Chromium path by stage, in one fresh Node process per frame × scale (spawned by
// stages.mjs): node stage-run.mjs <input.json> <scale> <fontBase> <out.json> <png-out>
// Startup replicates worker/runtime.mjs startRuntime step by step with timers (same allowlist,
// launch and page options). Jobs drive the real PipelineWorker (worker/pipeline.mjs) like
// crates/server/src/jobs/runner.rs: compute {strict:false}, then render {scale, strict:true}.
// Node-side timers wrap each layout-step spawnSync and page.evaluate round trip; page-side
// performance.now() timers wrap loadResources, measureBatch, drawScene and toDataURL.
const scriptStart = performance.now(); // ms since this process started
import { createServer } from "node:http";
import { readFileSync, writeFileSync } from "node:fs";
import path from "node:path";
import { PipelineWorker } from "../../../../worker/pipeline.mjs";
import { root } from "../../../../worker/runtime.mjs";
import { geometry } from "./geometry.mjs";

const [inputPath, scaleArg, fontBase, outPath, pngOut] = process.argv.slice(2);
const scale = Number(scaleArg);
const ITERS = Number(process.env.BENCH_ITERS ?? 25);
const base = JSON.parse(readFileSync(inputPath, "utf8"));
// Fonts come from a live release server (bytes, headers and SHA check as in production).
for (const f of base.fonts) f.url = `${fontBase}/api/v1/fonts/font_bundled`;
const now = () => performance.now();
const startup = {};
const step = async (name, fn) => {
  const s = now();
  const v = await fn();
  startup[name] = now() - s;
  return v;
};

const files = new Map([
  ["/renderer/dist/index.js", ["renderer/dist/index.js", "text/javascript"]],
  ["/renderer/fonts/NotoSans.ttf", ["renderer/fonts/NotoSans.ttf", "font/ttf"]],
]);
const server = await step("bundleServer", async () => {
  const s = createServer((req, res) => {
    const p = new URL(req.url, "http://localhost").pathname;
    if (p === "/") {
      res.writeHead(200, { "content-type": "text/html" });
      return res.end('<!doctype html><meta charset="utf-8"><canvas id="scene"></canvas>');
    }
    const f = files.get(p);
    if (!f) return res.writeHead(404).end();
    res.writeHead(200, { "content-type": f[1], "access-control-allow-origin": "*" });
    res.end(readFileSync(path.join(root, f[0])));
  });
  await new Promise((r) => s.listen(0, "127.0.0.1", r));
  return s;
});
const baseURL = `http://127.0.0.1:${server.address().port}`;
process.env.PLAYWRIGHT_BROWSERS_PATH ??= path.join(root, ".runtime/browsers");
const { chromium } = await step("importPlaywright", () => import("playwright"));
const browser = await step("launch", () => chromium.launch({ headless: true, timeout: 15_000 }));
const page = await step("newPage", () => browser.newPage({ viewport: { width: 800, height: 600 }, deviceScaleFactor: 1 }));
await step("goto", () => page.goto(baseURL, { timeout: 10_000 }));
await step("importRenderer", () => page.evaluate(async () => {
  globalThis.renderer = await import("/renderer/dist/index.js");
}));
startup.scriptStartMs = scriptStart;
startup.readyMs = now();

// Page-side timers. `renderer` becomes a plain object so PipelineWorker's evaluate bodies, which
// call renderer.loadResources/measureBatch/drawScene, reach the wrappers.
await page.evaluate(() => {
  const r = globalThis.renderer;
  const log = (globalThis.__bench = []);
  const sync = (name, fn) => (...a) => {
    const s = performance.now();
    try { return fn(...a); } finally { log.push([name, performance.now() - s]); }
  };
  globalThis.renderer = {
    ...r,
    measureBatch: sync("measureBatch", r.measureBatch),
    drawScene: sync("drawScene", r.drawScene),
    loadResources: async (...a) => {
      const s = performance.now();
      try { return await r.loadResources(...a); } finally { log.push(["loadResources", performance.now() - s]); }
    },
  };
  const toDataURL = HTMLCanvasElement.prototype.toDataURL;
  HTMLCanvasElement.prototype.toDataURL = function (...a) {
    const s = performance.now();
    const v = toDataURL.apply(this, a);
    log.push(["toDataURL", performance.now() - s, v.length]);
    return v;
  };
});

class TimedWorker extends PipelineWorker {
  log = { layout: [], evals: [], overhead: 0 };
  async resources(...a) {
    this.stage = "resources";
    try { return await super.resources(...a); } finally { this.stage = null; }
  }
  layoutStep(input, cache) {
    const s = now();
    const result = super.layoutStep(input, cache);
    const ms = now() - s;
    const o = now(); // byte counts are instrumentation: excluded from job totals
    this.log.layout.push({ ms, inBytes: Buffer.byteLength(JSON.stringify({ input, cache })) + 1,
      outBytes: Buffer.byteLength(JSON.stringify(result)) + 1 });
    this.log.overhead += now() - o;
    this.stage = "measure";
    return result;
  }
  async render(...a) {
    this.inRender = true;
    try { return await super.render(...a); } finally { this.inRender = false; }
  }
}
const runtime = { page, browser, baseURL, browserVersion: browser.version(),
  close: async () => { await browser.close(); await new Promise((r) => server.close(r)); } };
const worker = new TimedWorker(runtime);
const evaluate = page.evaluate.bind(page);
page.evaluate = async (fn, arg) => {
  const stage = worker.stage ?? (worker.inRender ? "draw" : "other");
  const s = now();
  try { return await evaluate(fn, arg); } finally { worker.log.evals.push({ stage, ms: now() - s }); }
};

const sum = (xs) => xs.reduce((a, b) => a + b, 0);
const timed = (fn) => { const s = now(); const v = fn(); return [v, now() - s]; };

async function job(k, nocache) {
  if (nocache) worker.cache = {};
  const input = { ...base, viewport: { width: base.viewport.width + k, height: base.viewport.height } };
  worker.log = { layout: [], evals: [], overhead: 0 };
  await evaluate(() => (globalThis.__bench.length = 0));
  let s = now();
  const computed = await worker.compute(input, { strict: false });
  const computeMs = now() - s - worker.log.overhead;
  s = now();
  const rendered = await worker.render(computed.scene, { scale, strict: true });
  const renderMs = now() - s;
  // worker/cli.mjs per job: parse each request line, stringify each response line.
  const computeReq = JSON.stringify({ op: "compute", input, strict: false, id: 1 });
  const renderReq = JSON.stringify({ op: "render", scene: computed.scene, scale, strict: true, id: 2 });
  const [, parseComputeReqMs] = timed(() => JSON.parse(computeReq));
  const [computeLine, stringifyComputeMs] = timed(() => JSON.stringify({ id: 1, ...computed }));
  const [, parseRenderReqMs] = timed(() => JSON.parse(renderReq));
  const [renderLine, stringifyRenderMs] = timed(() => JSON.stringify({ id: 2, ...rendered }));
  const pageLog = await evaluate(() => globalThis.__bench);
  const pageMs = (name) => pageLog.filter((e) => e[0] === name).map((e) => e[1]);
  const evals = (stage) => worker.log.evals.filter((e) => e.stage === stage).map((e) => e.ms);
  const draw = pageMs("drawScene"), toData = pageLog.find((e) => e[0] === "toDataURL");
  return {
    k, width: input.viewport.width, nocache, computeMs, renderMs, jobMs: computeMs + renderMs,
    passes: computed.metrics.layoutPasses, measured: computed.metrics.measuredItems,
    layout: { n: worker.log.layout.length, ms: sum(worker.log.layout.map((l) => l.ms)),
      inBytes: sum(worker.log.layout.map((l) => l.inBytes)), outBytes: sum(worker.log.layout.map((l) => l.outBytes)) },
    measure: { n: evals("measure").length, rttMs: sum(evals("measure")), pageMs: sum(pageMs("measureBatch")) },
    resources: { n: evals("resources").length, rttMs: sum(evals("resources")), pageMs: sum(pageMs("loadResources")) },
    draw: { rttMs: sum(evals("draw")), drawPageMs: sum(draw), toDataURLPageMs: toData?.[1] ?? 0,
      base64Bytes: rendered.base64.length },
    ndjson: { parseComputeReqMs, stringifyComputeMs, parseRenderReqMs, stringifyRenderMs,
      ms: parseComputeReqMs + stringifyComputeMs + parseRenderReqMs + stringifyRenderMs,
      computeReqBytes: computeReq.length, computeRespBytes: computeLine.length,
      renderReqBytes: renderReq.length, renderRespBytes: renderLine.length },
    geometry: nocache ? undefined : geometry(computed.scene),
    png: k === 0 && !nocache ? rendered.base64 : undefined,
  };
}

const out = { startup, browserVersion: runtime.browserVersion, node: process.version, scale, jobs: [], nocacheJobs: [] };
try {
  for (let k = 0; k < ITERS; k++) out.jobs.push(await job(k, false));
  for (let k = 0; k < ITERS; k++) out.nocacheJobs.push(await job(k, true));
} finally {
  await runtime.close();
}
writeFileSync(pngOut, Buffer.from(out.jobs[0].png, "base64"));
delete out.jobs[0].png;
writeFileSync(outPath, JSON.stringify(out));
