// Chrome side: real renderer bundle in headless Chromium. Two launches (default, --font-render-hinting=none).
// Outputs data/chrome_widths_<mode>.json, data/chrome_wrap_<mode>.json, out/chrome-<mode>-s<k>.png, timing.
import { createServer } from "node:http";
import { readFileSync, writeFileSync } from "node:fs";
import { fileURLToPath } from "node:url";
// Repo root is two levels above this spike; works from any checkout or worktree.
const REPO = fileURLToPath(new URL("../../../", import.meta.url));
process.env.PLAYWRIGHT_BROWSERS_PATH ??= REPO + ".runtime/browsers";
const { chromium } = await import(REPO + "node_modules/playwright/index.mjs");
const here = (p) => new URL(`../${p}`, import.meta.url);
const corpus = JSON.parse(readFileSync(here("data/corpus.json")));
const scene = JSON.parse(readFileSync(here("data/scene.json")));
const files = {
  "/renderer/dist/index.js": [REPO + "renderer/dist/index.js", "text/javascript"],
  "/renderer/fonts/NotoSans.ttf": [REPO + "renderer/fonts/NotoSans.ttf", "font/ttf"],
  "/asset.png": [here("data/asset.png"), "image/png"],
};
const server = createServer((req, res) => {
  const p = new URL(req.url, "http://x").pathname;
  if (p === "/") { res.writeHead(200, { "content-type": "text/html" });
    return res.end('<!doctype html><meta charset="utf-8"><canvas id="scene"></canvas>'); }
  const f = files[p];
  if (!f) { res.writeHead(404); return res.end(); }
  res.writeHead(200, { "content-type": f[1] }); res.end(readFileSync(f[0]));
});
await new Promise((r) => server.listen(0, "127.0.0.1", r));
const baseURL = `http://127.0.0.1:${server.address().port}`;
const modes = { default: [], nohint: ["--font-render-hinting=none"] };
const timing = {};
for (const [mode, args] of Object.entries(modes)) {
  const browser = await chromium.launch({ headless: true, args });
  try {
    const page = await browser.newPage({ viewport: { width: 800, height: 600 }, deviceScaleFactor: 1 });
    await page.goto(baseURL);
    const version = browser.version();
    await page.evaluate(async ({ fonts, assets }) => {
      globalThis.renderer = await import("/renderer/dist/index.js");
      globalThis.resources = await renderer.loadResources(fonts, assets, { strict: true });
    }, { fonts: scene.fonts, assets: scene.assets });
    // Part A: raw Canvas2D widths exactly as measurement.ts sets the context.
    const widths = await page.evaluate(({ strings, sizes, font }) => {
      const ctx = document.createElement("canvas").getContext("2d");
      const out = {};
      for (const s of sizes) {
        ctx.font = `${font.style} ${font.weight} ${s}px "${font.family}"`;
        ctx.fontKerning = "normal";
        out[s] = strings.map((t) => ctx.measureText(t).width);
      }
      return out;
    }, { strings: corpus.strings, sizes: corpus.sizes, font: scene.fonts[0] });
    writeFileSync(here(`data/chrome_widths_${mode}.json`), JSON.stringify(widths));
    // Part A.5: wrap via the real measureBatch.
    const reqs = [];
    corpus.paragraphs.forEach((p, pi) => corpus.wrap.sizes.forEach(([s, lh]) => corpus.wrap.widths.forEach((w) =>
      reqs.push({ key: `${pi}|${s}|${w}`, text: p, fontId: "font_bundled", fontHash: scene.fonts[0].hash,
        family: scene.fonts[0].family, weight: 400, style: "normal", fontSize: s, lineHeight: lh, width: w }))));
    const wrapped = await page.evaluate((r) => renderer.measureBatch(r, resources), reqs);
    writeFileSync(here(`data/chrome_wrap_${mode}.json`), JSON.stringify(wrapped));
    // Part B + C: drawScene + toDataURL at scale 1 and 2, then warm timing.
    for (const scale of [1, 2]) {
      const r = await page.evaluate(({ scene, scale }) => {
        const canvas = document.getElementById("scene");
        const once = () => { renderer.drawScene(canvas, scene, resources, { scale }); return canvas.toDataURL("image/png"); };
        const png = once();
        for (let i = 0; i < 5; i++) once();
        const N = 40; let t = performance.now();
        for (let i = 0; i < N; i++) once();
        const both = (performance.now() - t) / N;
        t = performance.now();
        for (let i = 0; i < N; i++) { renderer.drawScene(canvas, scene, resources, { scale }); canvas.getContext("2d").getImageData(0, 0, 1, 1); }
        const drawOnly = (performance.now() - t) / N;
        return { png: png.split(",")[1], both, drawOnly };
      }, { scene, scale });
      writeFileSync(here(`out/chrome-${mode}-s${scale}.png`), Buffer.from(r.png, "base64"));
      timing[`${mode}-s${scale}`] = { msDrawPlusToDataURL: r.both, msDrawPlusReadback1px: r.drawOnly, version };
    }
  } finally {
    await browser.close();
  }
}
server.close();
writeFileSync(here("data/chrome_timing.json"), JSON.stringify(timing, null, 1));
console.log(JSON.stringify(timing, null, 1));
