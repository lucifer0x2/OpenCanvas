// Task09 Chromium Web Worker gate. Run from any cwd; generated inputs/results stay in out/wasm.
import { createServer } from "node:http";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";
import { loadavg, cpus, totalmem, freemem } from "node:os";
import { runWorker } from "./wasm/worker-client.mjs";
import { validate } from "./wasm/metrics.mjs";

const REPO = fileURLToPath(new URL("../../../", import.meta.url));
process.env.PLAYWRIGHT_BROWSERS_PATH ??= REPO + ".runtime/browsers";
const { chromium } = await import(REPO + "node_modules/playwright/index.mjs");
const here = (p) => fileURLToPath(new URL(`../${p}`, import.meta.url));
const CASES = [["spike", 1], ["spike", 2], ["frame", 1], ["frame", 2], ["tile", 1], ["tile", 2]];
const variants = process.argv.slice(2).length ? process.argv.slice(2) : ["nosimd-release", "simd128-release"];
const inputs = new Map([
  ["/font.ttf", REPO + "renderer/fonts/NotoSans.ttf"],
  ["/asset.png", here("data/asset.png")],
  ["/worker.js", here("js/wasm/worker.js")],
  ...["case.mjs", "metrics.mjs"].map((f) => [`/${f}`, here(`js/wasm/${f}`)]),
]);
for (const v of variants) {
  if (!/^(nosimd|simd128)-release(?:-[sz])?$/.test(v)) throw new Error(`Invalid variant: ${v}`);
  inputs.set(`/rust_spike-${v}.wasm`, here(`out/wasm/rust_spike-${v}.wasm`));
}
for (const [name, scale] of CASES) {
  inputs.set(`/scenes/${name}.json`, here(`out/wasm/scenes/${name}.json`));
  inputs.set(`/native/${name}-s${scale}.rgba`, here(`out/wasm/native/${name}-s${scale}.rgba`));
}
// Fail before launch when any artifact cannot be read; no stale/missing case is silently skipped.
const inputHashes = {};
for (const [url, file] of inputs) {
  const bytes = readFileSync(file);
  if (!bytes.length) throw new Error(`Empty input: ${url}`);
  inputHashes[url] = createHash("sha256").update(bytes).digest("hex");
}
const nativeReport = JSON.parse(readFileSync(here("out/wasm/native.json")));
validate(nativeReport, CASES);
if (nativeReport.threads !== 1 || nativeReport.iters !== 20) throw new Error("Native must use --threads 1 --iters 20");
const types = { js: "text/javascript", mjs: "text/javascript", wasm: "application/wasm", json: "application/json", png: "image/png" };
const server = createServer((req, res) => {
  // Local-only isolation enables finer Worker performance.now() resolution, not WASM threads.
  res.setHeader("Cross-Origin-Opener-Policy", "same-origin");
  res.setHeader("Cross-Origin-Embedder-Policy", "require-corp");
  try {
    const p = new URL(req.url, "http://x").pathname;
    if (p === "/") {
      res.writeHead(200, { "content-type": "text/html" });
      return res.end("<!doctype html><title>Task09 Worker benchmark</title>");
    }
    const f = inputs.get(p);
    if (!f) { res.writeHead(404); return res.end("Not found"); }
    const bytes = readFileSync(f);
    res.writeHead(200, { "content-type": types[f.split(".").at(-1)] ?? "application/octet-stream" });
    res.end(bytes);
  } catch (error) {
    res.writeHead(500);
    res.end(String(error));
  }
});
let browser;
try {
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  browser = await chromium.launch({ headless: true });
  for (const v of variants) {
    const start = { at: new Date().toISOString(), loadavg: loadavg(), freeMemory: freemem() };
    const page = await browser.newPage();
    try {
      await page.goto(`http://127.0.0.1:${server.address().port}`);
      const result = await page.evaluate(runWorker, {
        wasm: `/rust_spike-${v}.wasm`, cases: CASES, iters: 20, timeoutMs: 180_000,
      });
      validate(result, CASES, true);
      result.browser = browser.version();
      result.variant = v;
      result.host = { platform: process.platform, arch: process.arch, node: process.version,
        cpu: cpus()[0]?.model, logicalCpus: cpus().length, totalMemory: totalmem(), start,
        end: { at: new Date().toISOString(), loadavg: loadavg(), freeMemory: freemem() } };
      result.inputHashes = inputHashes;
      writeFileSync(here(`out/wasm/chromium-${v}.json`), JSON.stringify(result, null, 2));
      console.log(v, JSON.stringify(result));
    } finally { await page.close(); }
  }
} finally {
  try { await browser?.close(); }
  finally {
    server.closeAllConnections();
    await new Promise((resolve, reject) => server.close((error) => {
      if (error && error.code !== "ERR_SERVER_NOT_RUNNING") reject(error);
      else resolve();
    }));
  }
}
