// Node opacity on text composites the node once, like CSS element opacity: where glyphs and
// decoration (or glyphs of negative tracking / tight lines) overlap, the result stays at the
// node's alpha. Canvas must match a Chromium CSS element with `opacity` and the HTML preview.
import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { build } from "esbuild";
import { readFile } from "node:fs/promises";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { textFonts, textInput } from "./text-fixtures.mjs";
let worker, fonts;
before(async () => {
  fonts = await textFonts();
  const bundle = await build({ entryPoints: ["web/src/preview/sceneDom.ts"], bundle: true, format: "esm", write: false });
  worker = await PipelineWorker.start({ resourceHandler(req, res) {
    if (req.url !== "/preview-adapter.js") return false;
    res.writeHead(200, { "content-type": "text/javascript" }); res.end(bundle.outputFiles[0].text); return true;
  } });
  await worker.runtime.page.addStyleTag({ content: `body{margin:0}${await readFile("web/src/preview/preview.css", "utf8")}` });
});
after(async () => { await worker?.close(); });

const opacityCases = {
  "line-through": { text: "HHHH", textDecoration: "line-through" },
  "underline through descenders": { text: "gjpqy", textDecoration: "underline" },
  "negative tracking": { text: "HHHH", letterSpacing: -20 },
  "tight lines": { text: "gjpq\nHHHH", lineHeight: 20 },
};
const base = { width: 380, fontSize: 40, lineHeight: 48, fill: "#000000" };

/** Darkness (255 - min RGB) per pixel of the worker canvas or of a PNG, plus raw RGBA. */
async function read(png) {
  return worker.runtime.page.evaluate(async (b64) => {
    let data;
    if (b64) {
      const i = new Image(); i.src = `data:image/png;base64,${b64}`; await i.decode();
      const c = document.createElement("canvas"); c.width = i.width; c.height = i.height;
      c.getContext("2d").drawImage(i, 0, 0); data = c.getContext("2d").getImageData(0, 0, i.width, i.height).data;
    } else data = document.getElementById("scene").getContext("2d").getImageData(0, 0, 400, 160).data;
    return [...data];
  }, png?.toString("base64"));
}

function diff(a, b) {
  let pixels = 0, maxDelta = 0;
  for (let p = 0; p < a.length; p += 4) {
    const d = Math.max(...[0, 1, 2, 3].map(i => Math.abs(a[p + i] - b[p + i])));
    if (d) pixels++; maxDelta = Math.max(maxDelta, d);
  }
  return { pixels, maxDelta };
}
const darkest = (rgba) => { let m = 0; for (let p = 0; p < rgba.length; p += 4) m = Math.max(m, 255 - Math.min(rgba[p], rgba[p + 1], rgba[p + 2])); return m; };

/** Canvas, the CSS element oracle (`opacity` on the text element) and the production preview. */
async function consumers(props) {
  const { scene } = await worker.compute(textInput(fonts, props));
  const { draw } = await worker.render(scene);
  const canvas = await read();
  await worker.runtime.page.evaluate(({ scene, p }) => {
    document.getElementById("scene").style.display = "none";
    const n = scene.nodes[1], font = scene.fonts.find(f => f.id === n.fontId);
    const oracle = document.createElement("div"); oracle.id = "oracle";
    oracle.style.cssText = "position:absolute;left:0;top:0;width:400px;height:160px;background:#fff";
    const text = document.createElement("div");
    text.style.cssText = `position:absolute;left:${n.bounds.x}px;top:${n.bounds.y}px;width:${n.bounds.width}px;font:${renderer.fontCSS(font, p.fontSize)};line-height:${p.lineHeight}px;letter-spacing:${p.letterSpacing ?? 0}px;text-decoration:${p.textDecoration ?? "none"};text-decoration-skip-ink:none;white-space:pre-line;color:${p.fill};font-kerning:normal;opacity:${p.opacity ?? 1}`;
    text.textContent = p.text; oracle.append(text); document.body.append(oracle);
  }, { scene, p: props });
  const css = await read(await worker.runtime.page.locator("#oracle").screenshot());
  await worker.runtime.page.evaluate(async (scene) => {
    document.getElementById("oracle").remove();
    const { buildSceneDom } = await import("/preview-adapter.js");
    const dom = buildSceneDom(scene, resources, new Set()); dom.root.id = "preview";
    dom.root.style.cssText += ";position:absolute;left:0;top:0"; document.body.append(dom.root);
  }, scene);
  const preview = await read(await worker.runtime.page.locator("#preview").screenshot());
  await worker.runtime.page.evaluate(() => document.getElementById("preview").remove());
  return { draw, canvas, css, preview, folded: await folded(scene) };
}

/** Negative control: the same scene with no layer budget, so node alpha folds into each paint. */
async function folded(scene) {
  return worker.runtime.page.evaluate((scene) => {
    const c = document.createElement("canvas"); c.width = 400; c.height = 160;
    const ctx = c.getContext("2d"); ctx.fillStyle = "#fff"; ctx.fillRect(0, 0, 400, 160);
    const stats = renderer.paintTree(ctx, scene, resources, renderer.paintNode, { maxLayerPixels: 0 });
    return { stats, rgba: [...ctx.getImageData(0, 0, 400, 160).data] };
  }, scene);
}

test("text node opacity composites glyphs and decoration once, like CSS element opacity", async () => {
  const results = [];
  for (const [name, props] of Object.entries(opacityCases)) {
    const r = await consumers({ ...base, ...props, opacity: 0.5 });
    const css = diff(r.canvas, r.css), preview = diff(r.canvas, r.preview);
    const dark = { canvas: darkest(r.canvas), css: darkest(r.css), preview: darkest(r.preview), folded: darkest(r.folded.rgba) };
    results.push({ name, layers: r.draw.opacityLayers, css, preview, dark });
    assert.equal(r.draw.opacityLayers, 1, name);
    // Black at alpha 0.5 over white is darkness 127.5; double coverage (the old per-paint alpha)
    // reaches 0.75 = 191. Overlap pixels are the darkest ones, so the maximum bounds them.
    assert.ok(dark.canvas <= 130 && Math.abs(dark.canvas - dark.css) <= 1, `${name} ${JSON.stringify(dark)}`);
    assert.ok(css.maxDelta <= 1, `${name} canvas vs CSS ${JSON.stringify(css)}`);
    assert.ok(preview.maxDelta <= 1, `${name} canvas vs preview ${JSON.stringify(preview)}`);
    assert.equal(r.folded.stats.opacityLayerFallbacks, 1, name);
    assert.ok(dark.folded >= 185, `${name} folded alpha must double-cover: ${JSON.stringify(dark)}`);
  }
  console.log(JSON.stringify(results));
});

test("negative control: per-paint alpha (translucent fill colour) double-covers overlaps, as CSS colour does", async () => {
  // The pre-fix canvas path folded node opacity into each fillText/fillRect, which is exactly a
  // translucent fill colour at opacity 1. CSS paints a translucent colour per glyph/decoration
  // too, so here overlap reaching ~0.75 is correct and must stay that way.
  const results = [];
  for (const [name, props] of Object.entries(opacityCases)) {
    const r = await consumers({ ...base, ...props, fill: "#00000080" });
    const dark = { canvas: darkest(r.canvas), css: darkest(r.css) };
    results.push({ name, layers: r.draw.opacityLayers, dark, css: diff(r.canvas, r.css) });
    assert.equal(r.draw.opacityLayers, 0, name);
    assert.ok(dark.canvas >= 185 && Math.abs(dark.canvas - dark.css) <= 1, `${name} ${JSON.stringify(dark)}`);
  }
  console.log(JSON.stringify(results));
});

test("translucent text without overlap keeps the direct (layerless) path", async () => {
  for (const props of [{ text: "HHHH" }, { text: "Hello\nworld", lineHeight: 56 }]) {
    const r = await consumers({ ...base, ...props, opacity: 0.5 });
    assert.equal(r.draw.opacityLayers, 0, JSON.stringify(props));
    assert.ok(diff(r.canvas, r.css).maxDelta <= 1, JSON.stringify(props));
  }
});
