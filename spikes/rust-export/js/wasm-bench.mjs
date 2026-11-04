// UNTESTED (Task 09): written on a Linux VM without Node/Chromium; never run. Browser gate runner:
// serves the spike over HTTP on port 0, runs js/wasm/worker.js in the repo's pinned Chromium
// (.runtime/browsers, Playwright from node_modules) for each .wasm variant, writes
// out/wasm/chromium-<variant>.json and closes browser and server. Inputs come from the Phần E commands
// (README): out/wasm/scenes/*.json (`frames`), out/wasm/native/*.rgba (`frame-bench --dump`) and
// out/wasm/rust_spike-*.wasm (wasm/build.sh). Usage: node spikes/rust-export/js/wasm-bench.mjs [variant...]
import { createServer } from "node:http";
import { existsSync, readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

const REPO = fileURLToPath(new URL("../../../", import.meta.url));
process.env.PLAYWRIGHT_BROWSERS_PATH ??= REPO + ".runtime/browsers";
const { chromium } = await import(REPO + "node_modules/playwright/index.mjs");
const here = (p) => fileURLToPath(new URL(`../${p}`, import.meta.url));
const CASES = [["spike", 1], ["spike", 2], ["frame", 1], ["frame", 2], ["tile", 1], ["tile", 2]];
const variants = process.argv.slice(2).length ? process.argv.slice(2) : ["nosimd-release", "simd128-release"];

const types = { ".js": "text/javascript", ".wasm": "application/wasm", ".json": "application/json", ".png": "image/png" };
const route = (p) => {
  if (p === "/font.ttf") return REPO + "renderer/fonts/NotoSans.ttf";
  if (p === "/asset.png") return here("data/asset.png");
  if (p === "/worker.js") return here("js/wasm/worker.js");
  const m = p.match(/^\/(scenes|native)\/([\w.-]+)$/) || p.match(/^\/()(rust_spike-[\w-]+\.wasm)$/);
  return m ? here(`out/wasm/${m[1] ? m[1] + "/" : ""}${m[2]}`) : null;
};
const server = createServer((req, res) => {
  const p = new URL(req.url, "http://x").pathname;
  if (p === "/") { res.writeHead(200, { "content-type": "text/html" }); return res.end("<!doctype html><title>wasm</title>"); }
  const f = route(p);
  if (!f || !existsSync(f)) { res.writeHead(404); return res.end(); }
  res.writeHead(200, { "content-type": types[f.slice(f.lastIndexOf("."))] ?? "application/octet-stream" });
  res.end(readFileSync(f));
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const baseURL = `http://127.0.0.1:${server.address().port}`;
const browser = await chromium.launch({ headless: true });
try {
  for (const v of variants) {
    const page = await browser.newPage();
    await page.goto(baseURL);
    const result = await page.evaluate(({ wasm, cases, iters }) => new Promise((resolve) => {
      const w = new Worker("/worker.js");
      w.onmessage = (e) => { w.terminate(); resolve(e.data); };
      w.postMessage({ wasm, cases, iters });
    }), { wasm: `/rust_spike-${v}.wasm`, cases: CASES, iters: 20 });
    result.browser = browser.version();
    writeFileSync(here(`out/wasm/chromium-${v}.json`), JSON.stringify(result, null, 1));
    console.log(v, JSON.stringify(result));
    await page.close();
  }
} finally {
  await browser.close();
  server.close();
}
