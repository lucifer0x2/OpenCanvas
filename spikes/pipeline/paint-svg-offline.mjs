// Production paint SVG (written by crates/server/tests/paint_svg.rs) against the CSS reference
// and the canvas, in Chromium at device scales 1 and 2, offline. Usage: <svg> <scene.json>.
import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { PipelineWorker } from "../../worker/pipeline.mjs";
const scene = JSON.parse(await readFile(process.argv[3], "utf8"));
const art = scene.nodes.find((n) => n.id === "art");
const nodes = scene.nodes.filter((n) => n.parentId === "art");
const px = (v) => `${v}px`;
function background(fill) {
  if (typeof fill === "string") return fill;
  const stops = fill.stops.map(([t, c]) => `${c} ${t * 100}%`).join(",");
  if (fill.type === "linear") return `linear-gradient(${fill.angle}deg,${stops})`;
  const r = fill.radius, shape = typeof r === "number" ? `circle ${r}px` : r ? `${r[0]}px ${r[1]}px` : "ellipse farthest-corner";
  return `radial-gradient(${shape} at ${fill.center[0] * 100}% ${fill.center[1] * 100}%,${stops})`;
}
const css = nodes.map((n) => {
  const b = n.bounds, radius = Array.isArray(n.radius) ? n.radius : [n.radius ?? 0];
  return `<div style="position:absolute;left:${px(b.x - art.bounds.x)};top:${px(b.y - art.bounds.y)};width:${px(b.width)};height:${px(b.height)};background:${background(n.fill)};border-radius:${radius.map(px).join(" ")}"></div>`;
}).join("");
const worker = await PipelineWorker.start();
// As paint-fixtures.mjs: SVG and CSS stay within rounded-corner AA ("edge"); the canvas fill of
// the 76px asymmetric arc antialiases differently from CSS at scale 2 ("arc").
const budgets = { svgVsCss: { pixels: 12, max: 48 }, svgVsCanvas: { pixels: 190, max: 96 }, canvasVsCss: { pixels: 190, max: 96 } };
try {
  for (const scale of [1, 2]) {
    const page = await worker.runtime.browser.newPage({ offline: true, viewport: { width: 480, height: 300 }, deviceScaleFactor: scale });
    const network = [];
    page.on("request", (r) => { if (/^https?:/.test(r.url())) network.push(r.url()); });
    await page.goto(pathToFileURL(process.argv[2]).href);
    await page.evaluate(() => { document.documentElement.style.margin = "0"; });
    const svg = await page.locator("svg").screenshot();
    await page.close();
    const html = await worker.runtime.browser.newPage({ viewport: { width: 480, height: 300 }, deviceScaleFactor: scale });
    await html.setContent(`<!doctype html><body style="margin:0;background:#fff">${css}`);
    const reference = await html.screenshot();
    await html.close();
    assert.deepEqual(network, []);
    await worker.render(scene, { scale });
    const result = await worker.runtime.page.evaluate(async ({ svg, reference, w, h }) => {
      const read = async (b64) => { const i = new Image(); i.src = `data:image/png;base64,${b64}`; await i.decode(); const c = document.createElement("canvas"); c.width = w; c.height = h; const x = c.getContext("2d"); x.drawImage(i, 0, 0); return x.getImageData(0, 0, w, h).data; };
      const compare = (a, b) => { let max = 0, pixels = 0, at = []; for (let p = 0; p < a.length; p += 4) { let d = 0; for (let k = 0; k < 4; k++) d = Math.max(d, Math.abs(a[p + k] - b[p + k])); max = Math.max(max, d); if (d > 2) { pixels++; if (at.length < 3) at.push([(p / 4) % w, Math.floor(p / 4 / w)]); } } return { pixels, max, at }; };
      const s = await read(svg), r = await read(reference);
      const canvas = document.getElementById("scene").getContext("2d").getImageData(0, 0, w, h).data;
      return { svgVsCss: compare(s, r), svgVsCanvas: compare(s, canvas), canvasVsCss: compare(canvas, r) };
    }, { svg: svg.toString("base64"), reference: reference.toString("base64"), w: 480 * scale, h: 300 * scale });
    console.log(`scale ${scale}: ${JSON.stringify(result)}`);
    for (const [k, v] of Object.entries(result))
      assert.ok(v.pixels <= budgets[k].pixels * scale && v.max <= budgets[k].max, `${k}@${scale} ${JSON.stringify(v)}`);
  }
} finally {
  await worker.close();
}
