import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { textFonts, textInput, copy } from "./text-fixtures.mjs";
let worker, fonts;
before(async () => { fonts = await textFonts(); worker = await PipelineWorker.start(); await worker.resources(fonts, []); });
after(async () => { await worker?.close(); });

test("pinned Noto and Lato: ceil64 max-content is stable and baseline equals DOM half-leading", async () => {
  const errors = await worker.runtime.page.evaluate(({ fonts, copy }) => {
    const errors = [];
    for (const font of fonts) for (const fontSize of [11, 12, 13, 14, 16, 24]) for (const text of copy) for (const letterSpacing of [-0.5, 0, 1.25]) {
      const r = { key: "test", fontId: font.id, fontHash: font.hash, family: font.family, weight: font.weight, style: font.style, fontSize, lineHeight: fontSize + 6, text, letterSpacing, width: null };
      const measured = renderer.measureBatch([r], resources).test;
      const repeated = renderer.measureBatch([{ ...r, width: measured.width }], resources).test;
      const div = document.createElement("div");
      div.style.cssText = `position:absolute;width:max-content;white-space:pre;font:${renderer.fontCSS(font, fontSize)};line-height:${r.lineHeight}px;letter-spacing:${letterSpacing}px;font-kerning:normal`;
      div.textContent = text;
      const marker = document.createElement("i");
      marker.style.cssText = "display:inline-block;width:0;height:0";
      div.append(marker); document.body.append(div);
      const baseline = marker.getBoundingClientRect().top - div.getBoundingClientRect().top;
      const width = div.getBoundingClientRect().width;
      div.remove();
      if (repeated.lines.length !== 1 || repeated.width !== measured.width || width !== measured.width || baseline !== measured.lines[0].y)
        errors.push({ font: font.id, text, fontSize, letterSpacing, width, baseline, measured, repeated });
    }
    return errors.slice(0, 10);
  }, { fonts, copy });
  assert.deepEqual(errors, []);
});

test("wrap respects NBSP/narrow NBSP and graphemes; nowrap preserves forced LF", async () => {
  const scene = async (text, props = {}) => (await worker.compute(textInput(fonts, { text, width: 1, ...props }))).scene.nodes[1];
  assert.deepEqual((await scene("a\u00a0b")).lines.map(l => l.text), ["a\u00a0b"]);
  assert.deepEqual((await scene("a\u202fb")).lines.map(l => l.text), ["a\u202fb"]);
  assert.deepEqual((await scene("a\u2060b")).lines.map(l => l.text), ["a\u2060b"]);
  assert.deepEqual((await scene("A\u0306\u0301👩‍💻🇻🇳")).lines.map(l => l.text), ["A\u0306\u0301", "👩‍💻", "🇻🇳"]);
  assert.deepEqual((await scene("  a\t b\nc d  ", { textWrap: "nowrap" })).lines.map(l => l.text), ["a b", "c d"]);
  assert.equal((await scene("customers", { width: "hug", x: 360, textWrap: "nowrap" })).bounds.height, 20);
  assert.ok((await scene("customers", { width: "hug", x: 360 })).lines.length > 1);
  const boundary = await scene("Prefer", { width: "hug", x: 10 });
  const exact = await scene("Prefer", { width: boundary.lines[0].width });
  const below = await scene("Prefer", { width: boundary.lines[0].width - 1 / 64 });
  assert.equal(exact.lines.length, 1); assert.equal(below.lines.length, 2);
});

test("short, negative-leading and fractional baselines equal DOM; signed tracking changes cache and boundary wrap", async () => {
  const errors = await worker.runtime.page.evaluate(({ fonts }) => {
    const errors = [];
    for (const font of fonts) for (const [fontSize, lineHeight] of [[11, 13], [12, 18], [13, 16], [14, 20], [15, 15], [16, 24], [32, 40]]) {
      const r = { key: "baseline", fontId: font.id, fontHash: font.hash, family: font.family, weight: 400, style: "normal", fontSize, lineHeight, text: "Hxg Sao chép Ấp\nsecond line ỹ", width: null };
      const m = renderer.measureBatch([r], resources).baseline;
      const div = document.createElement("div");
      div.style.cssText = `position:absolute;white-space:pre;font:${renderer.fontCSS(font, fontSize)};line-height:${lineHeight}px`;
      const marker = document.createElement("i"); marker.style.cssText = "display:inline-block;width:0;height:0";
      div.append("Hxg Sao chép Ấp", marker); document.body.append(div);
      const baseline = marker.getBoundingClientRect().top - div.getBoundingClientRect().top; div.remove();
      if (m.lines[0].y !== baseline || m.lines[1].y - m.lines[0].y !== lineHeight) errors.push({ font: font.id, fontSize, lineHeight, baseline, m });
    }
    return errors;
  }, { fonts });
  assert.deepEqual(errors, []);
  for (const letterSpacing of [-1, 0, 2]) {
    const input = textInput(fonts, { text: "Tracking boundary", width: "hug", textWrap: "nowrap", letterSpacing });
    const first = await worker.compute(input), width = first.scene.nodes[1].bounds.width;
    const exact = await worker.compute(textInput(fonts, { text: "Tracking boundary", width, letterSpacing }));
    assert.equal(exact.scene.nodes[1].lines.length, 1);
    assert.equal((await worker.compute(input)).metrics.batchRPCs, 0);
  }
});

test("coverage warns from pinned cmap, preserves geometry, composes only for diagnostics", async () => {
  const measure = async (text) => (await worker.compute(textInput(fonts, { text }))).scene;
  for (const text of ["Tiếng Việt ă â đ ê ô ơ ư", "Tiếng Việt ă â đ ê ô ơ ư".normalize("NFD")]) {
    const s = await measure(text); assert.deepEqual(s.diagnostics, []);
  }
  const s = await measure("⌘C 中 🦄");
  assert.ok(s.diagnostics.some(d => d.includes("MISSING_GLYPH node=text font=font_bundled") && d.includes("U+4E2D")), JSON.stringify(s.diagnostics));
  const geometry = await worker.runtime.page.evaluate(async ({ fonts }) => {
    const r = { key: "x", text: "⌘C 中 🦄", fontId: fonts[0].id, fontHash: fonts[0].hash, family: fonts[0].family, weight: 400, style: "normal", fontSize: 14, lineHeight: 20, width: 300 };
    const yes = renderer.measureBatch([r], resources).x;
    const no = renderer.measureBatch([r], { ...resources, glyphCoverage: undefined }).x;
    return [yes.lines, no.lines, yes.width, no.width, no.coverageUnavailable];
  }, { fonts });
  assert.deepEqual(geometry[0], geometry[1]); assert.equal(geometry[2], geometry[3]); assert.equal(geometry[4], true);
});

test("case transforms preserve authored source and align final measured lines", async () => {
  for (const textAlign of ["center", "right"]) {
    const input = textInput(fonts, { text: "Straße sao chép\nhello-world o'neil", textTransform: "capitalize", textAlign, letterSpacing: -0.5, textDecoration: "line-through" });
    const original = JSON.stringify(input);
    const result = await worker.compute(input), n = result.scene.nodes[1];
    assert.equal(JSON.stringify(input), original);
    assert.equal(n.lines[0].text, "Straße Sao Chép");
    assert.equal(n.lines[1].text, "Hello-World O'neil");
    assert.equal(n.letterSpacing, -0.5); assert.equal(n.decoration.line, "line-through");
    for (const line of n.lines) assert.equal(line.x, textAlign === "center" ? Math.floor((n.bounds.width - line.width) * 32) / 64 : n.bounds.width - line.width);
    const repeat = await worker.compute(input); assert.equal(repeat.metrics.batchRPCs, 0);
  }
  const upper = (await worker.compute(textInput(fonts, { text: "Straße ﬁ", textTransform: "uppercase" }))).scene.nodes[1];
  assert.equal(upper.lines[0].text, "STRASSE FI");
});
