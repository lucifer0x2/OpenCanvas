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

async function compareScreenshot(png, scene, cssOracle = false) {
  return worker.runtime.page.evaluate(async ({ base64, scene, cssOracle }) => {
    const image = new Image(); image.src = `data:image/png;base64,${base64}`; await image.decode();
    const c = document.createElement("canvas"); c.width = image.width; c.height = image.height;
    c.getContext("2d").drawImage(image, 0, 0);
    const expected = document.getElementById("scene").getContext("2d").getImageData(0, 0, c.width, c.height).data;
    const actual = c.getContext("2d").getImageData(0, 0, c.width, c.height).data;
    let pixels = 0, maxDelta = 0, unexpected = 0; const sample = [];
    for (let p = 0; p < actual.length; p += 4) if ([0, 1, 2, 3].some(i => expected[p + i] !== actual[p + i])) {
      const delta = Math.max(...[0, 1, 2, 3].map(i => Math.abs(expected[p + i] - actual[p + i])));
      const x = p / 4 % c.width, y = Math.floor(p / 4 / c.width);
      // Chrome CSS composites the underline through descender antialiasing with
      // occasional one-channel rounding. Allow only opaque interior pixels on
      // the exact underline rectangle; endpoints and every other pixel stay exact.
      const overlap = cssOracle && delta <= 1 && scene.nodes.some(n => n.decoration?.line === "underline" && n.lines.some(l => {
        const top = Math.round(n.bounds.y + l.y + n.decoration.offset);
        return x > n.bounds.x + l.x && x + 1 < n.bounds.x + l.x + l.width && y >= top && y < top + n.decoration.thickness;
      })) && expected[p + 3] === 255 && expected[p] === 24 && expected[p + 1] === 34 && expected[p + 2] === 48;
      pixels++; maxDelta = Math.max(maxDelta, delta); if (!overlap) unexpected++;
      if (sample.length < 12) sample.push({ x, y, expected: [...expected.slice(p, p + 4)], actual: [...actual.slice(p, p + 4)] });
    }
    return { pixels, maxDelta, unexpected, sample };
  }, { base64: png.toString("base64"), scene, cssOracle });
}

async function checkPixels(input, checkLines = false) {
    const { scene } = await worker.compute(input);
    await worker.render(scene);
    await worker.runtime.page.evaluate(({ scene, input }) => {
      document.getElementById("scene").style.display = "none";
      const n = scene.nodes[1], p = input.nodes[1].props, font = scene.fonts.find(f => f.id === n.fontId);
      const oracle = document.createElement("div"); oracle.id = "oracle";
      oracle.style.cssText = "position:absolute;left:0;top:0;width:400px;height:160px;background:#fff";
      const text = document.createElement("div");
      text.id = "oracle-text";
      text.style.cssText = `position:absolute;left:${n.bounds.x}px;top:${n.bounds.y}px;width:${n.bounds.width}px;font:${renderer.fontCSS(font, p.fontSize)};line-height:${p.lineHeight}px;letter-spacing:${p.letterSpacing ?? 0}px;text-align:${p.textAlign ?? "left"};text-decoration:${p.textDecoration ?? "none"};text-transform:${p.textTransform ?? "none"};text-decoration-skip-ink:none;white-space:pre-line;color:${p.fill};font-kerning:normal`;
      text.textContent = p.text; oracle.append(text); document.body.append(oracle);
    }, { scene, input });
    if (checkLines) {
      const lines = await worker.runtime.page.evaluate(() => {
        const text = document.getElementById("oracle-text"), child = text.firstChild;
        const lines = []; let top;
        for (let i = 0; i < child.length; i++) {
          const range = document.createRange(); range.setStart(child, i); range.setEnd(child, i + 1);
          const box = range.getBoundingClientRect();
          if (top !== box.top) { lines.push(""); top = box.top; }
          lines[lines.length - 1] += child.textContent[i];
        }
        return lines.map(l => l.trim().toUpperCase()).filter(Boolean);
      });
      assert.deepEqual(scene.nodes[1].lines.map(l => l.text), lines);
    }
    const css = await worker.runtime.page.locator("#oracle").screenshot();
    const cssDiff = await compareScreenshot(css, scene, true);
    assert.equal(cssDiff.unexpected, 0, JSON.stringify({ props: input.nodes[1].props, cssDiff }));
    await worker.runtime.page.evaluate(async (scene) => {
      document.getElementById("oracle").remove();
      const { buildSceneDom } = await import("/preview-adapter.js");
      const dom = buildSceneDom(scene, resources, new Set()); dom.root.id = "preview";
      dom.root.style.cssText += ";position:absolute;left:0;top:0"; document.body.append(dom.root);
    }, scene);
    const preview = await worker.runtime.page.locator("#preview").screenshot();
    const previewDiff = await compareScreenshot(preview, scene);
    assert.equal(previewDiff.pixels, 0, JSON.stringify({ props: input.nodes[1].props, previewDiff }));
    await worker.runtime.page.evaluate(() => document.getElementById("preview").remove());
    return [cssDiff, previewDiff];
}

test("canvas and production preview DOM match CSS oracle pixels for tracking, baselines, align and decoration", async () => {
  const differences = [];
  for (const font of fonts) for (const [fontSize, lineHeight] of [[14, 20], [24, 32]]) for (const letterSpacing of [-0.5, 0, 1.25]) for (const [textAlign, textDecoration] of [["left", "none"], ["center", "none"], ["right", "none"], ["center", "underline"], ["right", "line-through"]]) {
    const input = textInput(fonts, { fontId: font.id, text: "Sao chép Ấp\nsecond line ỹ", fontSize, lineHeight, letterSpacing, textAlign, textDecoration });
    const [css, preview] = await checkPixels(input);
    differences.push({ consumer: "css", font: font.id, fontSize, letterSpacing, textAlign, textDecoration, ...css }, { consumer: "preview", font: font.id, fontSize, letterSpacing, textAlign, textDecoration, ...preview });
  }
  console.log(JSON.stringify({ comparisons: differences.length, nonIdentical: differences.filter(d => d.pixels).map(({ sample, ...d }) => d) }));
});

test("CSS decoration metric matrix and transformed three-line paragraph retain exact geometry", async () => {
  const differences = [];
  for (const fontSize of [10, 12, 14, 16, 20, 24, 32, 40]) for (const ratio of [1.25, 1.5]) for (const textDecoration of ["underline", "line-through"]) {
    const [css, preview] = await checkPixels(textInput(fonts, { text: "Hello world HEAD Mixed", width: 380, fontSize, lineHeight: Math.round(fontSize * ratio), textDecoration }));
    differences.push({ fontSize, ratio, textDecoration, css: css.pixels, preview: preview.pixels });
  }
  const [css, preview] = await checkPixels(textInput(fonts, { text: "Sao chép tiếng Việt để đọc rõ trong paragraph", width: 160, fontSize: 14, lineHeight: 20, letterSpacing: 1, textAlign: "center", textTransform: "uppercase", textDecoration: "line-through" }), true);
  differences.push({ paragraph: true, css: css.pixels, preview: preview.pixels });
  console.log(JSON.stringify({ comparisons: differences.length * 2, nonIdentical: differences.filter(d => d.css || d.preview) }));
});

test("group opacity layer keeps large negative-tracking ink outside the advance box", async () => {
  const input = (opacity) => {
    const base = textInput(fonts, { x: 250, y: 50, width: "hug", fontSize: 24, lineHeight: 32, letterSpacing: -30, text: "ABCDEFGHIJ", fill: "#000000" });
    const text = { ...base.nodes[1], parentId: "g" };
    // The second child forces a layer (2 paint units); it sits right/below so it cannot widen the layer leftwards.
    // Negative control (inkBounds stripped from the scene) crops: max deviation 127.5.
    const group = { id: "g", parentId: "frame", kind: "group", props: { x: 0, y: 0, width: 400, height: 160, opacity, layout: { mode: "free" } } };
    const rect = { id: "r", parentId: "g", kind: "rectangle", props: { x: 370, y: 130, width: 10, height: 10, fill: "#000000" } };
    return { ...base, nodes: [base.nodes[0], group, rect, text] };
  };
  const darkness = async (opacity) => {
    const { scene } = await worker.compute(input(opacity));
    const { draw } = await worker.render(scene);
    const data = await worker.runtime.page.evaluate(() => [...document.getElementById("scene").getContext("2d").getImageData(0, 0, 400, 160).data]);
    return { scene, draw, dark: Array.from({ length: data.length / 4 }, (_, i) => 255 - Math.min(data[4 * i], data[4 * i + 1], data[4 * i + 2])) };
  };
  const solid = await darkness(1), layered = await darkness(0.5);
  assert.equal(layered.draw.opacityLayers, 1);
  const text = layered.scene.nodes.find(n => n.id === "text"), line = text.lines[0];
  // Fallback extent without inkBounds reaches only fontSize/lineHeight past the line box.
  const fallbackLeft = text.bounds.x + line.x - Math.max(text.fontSize, text.lineHeight);
  assert.ok(text.bounds.x + line.x + line.inkBounds.x < fallbackLeft - 50, JSON.stringify(line));
  let outside = 0, worst = 0;
  solid.dark.forEach((d, i) => {
    if (i % 400 < fallbackLeft && d > 0 && Math.floor(i / 400) > 30) outside++;
    worst = Math.max(worst, Math.abs(layered.dark[i] - d / 2));
  });
  assert.ok(outside > 100, `ink outside fallback extent: ${outside}`);
  // Layer alpha and the 0.5 composite each round to 8 bits: at most 1.5 darkness apart.
  assert.ok(worst <= 1.5, `max |layered - solid/2| darkness: ${worst}`);
  console.log(JSON.stringify({ inkLeft: text.bounds.x + line.x + line.inkBounds.x, fallbackLeft, outside, worst }));
});

test("overwide nowrap center/right match Chrome start alignment and LF remains separate", async () => {
  for (const textAlign of ["center", "right"]) {
    const { scene } = await worker.compute(textInput(fonts, { width: 20, text: "customers\ncustomers", textWrap: "nowrap", textAlign }));
    const result = await worker.runtime.page.evaluate(({ scene, textAlign }) => {
      const node = scene.nodes[1], font = scene.fonts[0];
      const div = document.createElement("div"); div.style.cssText = `width:20px;white-space:nowrap;text-align:${textAlign};font:${renderer.fontCSS(font, 14)}`;
      const span = document.createElement("span"); span.textContent = "customers"; div.append(span); document.body.append(div);
      const delta = span.getBoundingClientRect().x - div.getBoundingClientRect().x; div.remove(); return [delta, node.lines.map(l => l.x)];
    }, { scene, textAlign });
    assert.deepEqual(result, [0, [0, 0]]);
  }
});
