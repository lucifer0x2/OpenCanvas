// Production SVG of translucent overlapping text (written by
// crates/server/tests/text_opacity_svg.rs) against the canvas, offline in Chromium.
// Usage: <svg> <scene.json>. Negative control: the same markup with node opacity moved back onto
// each `<text>`/`<rect>` (the pre-fix form) double-covers the overlaps.
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { pathToFileURL } from "node:url";
import { PipelineWorker } from "../../worker/pipeline.mjs";
const [svgPath, scenePath] = process.argv.slice(2);
const scene = JSON.parse(await readFile(scenePath, "utf8"));
for (const font of scene.fonts)
  font.url = `data:font/ttf;base64,${(await readFile("renderer/fonts/NotoSans.ttf")).toString("base64")}`;
const markup = await readFile(svgPath, "utf8");
const perElement = markup.replace(/<g opacity="([\d.]+)">(.*?)<\/g>/g, (_, a, inner) =>
  inner.replace(/<(text|rect) /g, `<$1 opacity="${a}" `));
assert.notEqual(perElement, markup);
const oldPath = svgPath.replace(/\.svg$/, "-per-element.svg");
await writeFile(oldPath, perElement);
const { width, height } = scene.nodes[0].bounds;
const worker = await PipelineWorker.start();
try {
  await worker.resources(scene.fonts, []);
  await worker.render(scene);
  const canvas = await worker.runtime.page.evaluate(() => document.getElementById("scene").toDataURL());
  const shot = async (file) => {
    const page = await worker.runtime.browser.newPage({ offline: true, viewport: { width, height } });
    const network = [];
    page.on("request", (r) => { if (/^https?:/.test(r.url())) network.push(r.url()); });
    await page.goto(pathToFileURL(file).href);
    await page.evaluate(async () => { await document.fonts.ready; document.documentElement.style.margin = "0"; });
    const png = await page.locator("svg").screenshot();
    await page.close();
    assert.deepEqual(network, []);
    return png.toString("base64");
  };
  const svg = await shot(svgPath), old = await shot(oldPath);
  const result = await worker.runtime.page.evaluate(async ({ canvas, svg, old }) => {
    const decode = async (src) => { const i = new Image(); i.src = src; await i.decode(); const c = document.createElement("canvas"); c.width = i.width; c.height = i.height; c.getContext("2d").drawImage(i, 0, 0); return c.getContext("2d").getImageData(0, 0, i.width, i.height).data; };
    const a = await decode(canvas), b = await decode(`data:image/png;base64,${svg}`), c = await decode(`data:image/png;base64,${old}`);
    const darkest = (d) => { let m = 0; for (let p = 0; p < d.length; p += 4) m = Math.max(m, 255 - Math.min(d[p], d[p + 1], d[p + 2])); return m; };
    let pixels = 0, maxDelta = 0;
    for (let p = 0; p < a.length; p += 4) {
      const d = Math.max(...[0, 1, 2, 3].map((i) => Math.abs(a[p + i] - b[p + i])));
      if (d) pixels++; maxDelta = Math.max(maxDelta, d);
    }
    return { pixels, maxDelta, dark: { canvas: darkest(a), svg: darkest(b), perElement: darkest(c) } };
  }, { canvas, svg, old });
  console.log(JSON.stringify(result));
  // Black at node opacity 0.5 over white is darkness ~128; double coverage reaches ~192.
  assert.ok(result.maxDelta <= 1, `svg vs canvas ${JSON.stringify(result)}`);
  assert.ok(result.dark.svg <= 130 && result.dark.canvas <= 130, JSON.stringify(result.dark));
  assert.ok(result.dark.perElement >= 185, `negative control ${JSON.stringify(result.dark)}`);
} finally {
  await worker.close();
}
