import { readFile } from "node:fs/promises";
import { ROOT, sha256 } from "./environment.mjs";

export function authoredCases() {
  const cases = [
    ["empty", ""], ["space", " \t\f\v "], ["cr", "one\rtwo\r\nthree\nfour\n"],
    ["newlines", "\n\nTiếng Việt\n\n"], ["horizontal", " \tOne\f\v two\tthree  "],
    ["nbsp", "A\u00a0B C"], ["narrow-nbsp", "A\u202fB C"], ["joiner", "A\u2060B C"],
    ["bom", "A\ufeffB C"], ["nbsp-edge", "\u00a0A\u00a0"], ["bom-edge", "\ufeffA\ufeff"],
    ["unicode-space", "a\u2003b\u2009c\u2028d\u2029e"], ["zwsp", "a\u200bb"],
    ["vietnamese", "Tiếng Việt: ắ ằ ẳ ẵ ặ ể ễ ộ ợ ữ ỹ Nguyễn"],
    ["nfd", "Nguyễn ắ ậ ễ ợ ữ"], ["graphemes", "a\u0301\u0323b\u0308c\u0327"],
    ["long", "Supercalifragilisticexpialidocious"], ["ligatures", "office affinity ffi ffl fi fl"],
    ["kern", "AV To Ty We Yo VA WAVE Ta Tô Tư"], ["ink", "ẮgjÝỳ ÁÉÍÓÚ"],
    ["uppercase", "Straße ﬃ ı ǰ Ελληνικά", "uppercase"],
    ["lowercase", "ΟΣ ΟΣΑ Σ ΣΟΣ İ ẞ", "lowercase"],
    ["capitalize", "ßeta ﬃord 'hello' foo-bar tiếng việt οΣ", "capitalize"],
    ["case-newlines", "straße\r\nnguyễn văn\tΟΣ", "uppercase"],
    ["missing", "⌘ 漢字 \u{10ffff}", "none", true],
    ["emoji-graphemes", "👩‍💻 🇻🇳 1️⃣", "none", true],
  ];
  const out = [];
  for (const [label, text, textTransform = "none", fallback = false] of cases)
    for (const width of [null, 0, 23, 120]) out.push({ label: `${label}/${width}`, text, textTransform,
      width, fontSize: 16, lineHeight: 23, letterSpacing: 0, fallback });
  for (const letterSpacing of [-2, -0.5, 0, 0.5, 2])
    for (const fontSize of [11, 16, 32])
      for (const text of ["AV To WAVE ffi", "Tiếng Việt ắ", "jjff gy"])
        out.push({ label: `tracking/${letterSpacing}/${fontSize}/${text}`, text, fontSize,
          lineHeight: fontSize + 7.375, width: null, letterSpacing, textTransform: "none", fallback: false });
  // Odd/fractional/undersized line heights distinguish the font-bbox baseline from the old fontSize baseline.
  for (const lineHeight of [0, 9.25, 16, 21, 24.375]) out.push({ label: `baseline/${lineHeight}`,
    text: "Ắgj\nType", fontSize: 17.5, lineHeight, width: null, letterSpacing: 0, textTransform: "none", fallback: false });
  return out;
}

export async function buildCorpus(fonts) {
  const path = "spikes/rust-export/data/corpus.json";
  const bytes = await readFile(`${ROOT}${path}`);
  const legacy = JSON.parse(bytes);
  const requests = [], cases = {};
  const noto = fonts.find((f) => f.manifest.id === "noto").manifest;
  const add = (key, font, input, group, fallback = false) => {
    const { id: fontId, hash: fontHash, family, weight, style } = font;
    requests.push({ key, text: input.text, fontId, fontHash, family, weight, style,
      fontSize: input.fontSize, lineHeight: input.lineHeight, width: input.width,
      letterSpacing: input.letterSpacing ?? 0, textTransform: input.textTransform ?? "none" });
    cases[key] = { group, label: input.label ?? key, declaredFallback: fallback, thresholdOf: null };
  };
  legacy.strings.forEach((text, i) => legacy.sizes.forEach((fontSize) =>
    add(`spike/advance/${i}/${fontSize}`, noto, { text, fontSize, lineHeight: fontSize + 6, width: null }, "spike-advance")));
  legacy.paragraphs.forEach((text, i) => legacy.wrap.sizes.forEach(([fontSize, lineHeight]) =>
    legacy.wrap.widths.forEach((width) => add(`spike/wrap/${i}/${fontSize}/${width}`, noto,
      { text, fontSize, lineHeight, width }, "spike-wrap"))));
  for (const f of fonts) for (const [index, c] of authoredCases().entries())
    add(`current/${f.manifest.id}/${index}`, f.manifest, c, "current", c.fallback);
  return { requests, cases, source: { path, sha256: sha256(bytes),
    advanceCases: legacy.strings.length * legacy.sizes.length,
    wrapCases: legacy.paragraphs.length * legacy.wrap.sizes.length * legacy.wrap.widths.length } };
}

export function thresholdCases(requests, measurements, cases) {
  const extra = [];
  for (const r of requests) {
    if (!r.key.startsWith("current/") || r.width !== null || cases[r.key].declaredFallback) continue;
    const measurement = measurements[r.key];
    if (measurement.lines.length !== 1 || measurement.width <= 0 || measurement.missing.length) continue;
    for (const [name, delta] of [["below", -1 / 64], ["exact", 0], ["above", 1 / 64]]) {
      const key = `${r.key}/ceil64-${name}`;
      extra.push({ ...r, key, width: measurement.width + delta });
      cases[key] = { group: "ceil64-boundary", label: name, declaredFallback: false, thresholdOf: r.key };
    }
  }
  return extra;
}
