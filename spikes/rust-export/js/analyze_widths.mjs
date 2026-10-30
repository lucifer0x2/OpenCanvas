// Part A analysis: Chrome measureText widths vs HarfRust widths (and Chrome default vs nohint).
import { readFileSync, writeFileSync } from "node:fs";
const here = (p) => new URL(`../${p}`, import.meta.url);
const load = (p) => JSON.parse(readFileSync(here(p)));
const corpus = load("data/corpus.json");
const rust = load("data/rust_widths.json");
const round = (v) => Math.round(v * 64) / 64;
const ceil = (v) => Math.ceil(v * 64) / 64;
const report = {};
for (const mode of ["default", "nohint"]) {
  const chrome = load(`data/chrome_widths_${mode}.json`);
  const rows = [];
  for (const s of corpus.sizes) corpus.strings.forEach((t, i) => {
    const c = chrome[s][i], r = rust[s][i];
    rows.push({ text: t, size: s, chrome: c, rust: r, d: Math.abs(c - r),
      roundDiff: round(c) !== round(r), ceilDiff: ceil(c) !== ceil(r) });
  });
  const ds = rows.map((r) => r.d).sort((a, b) => a - b);
  const p = (q) => ds[Math.min(ds.length - 1, Math.floor(q * ds.length))];
  report[mode] = {
    pairs: rows.length, maxAbs: ds.at(-1), p99: p(0.99), p50: p(0.5), exactEqual: rows.filter((r) => r.d === 0).length,
    roundDiffers: rows.filter((r) => r.roundDiff).length, ceilDiffers: rows.filter((r) => r.ceilDiff).length,
    worst10: [...rows].sort((a, b) => b.d - a.d).slice(0, 10).map((r) => ({ text: r.text, size: r.size,
      chrome: r.chrome, rust: r.rust, delta: r.chrome - r.rust, chromeX64: r.chrome * 64, rustX64: r.rust * 64 })),
    roundDiffExamples: rows.filter((r) => r.roundDiff).slice(0, 10).map((r) => ({ text: r.text, size: r.size, chrome: r.chrome, rust: r.rust })),
  };
}
const a = load("data/chrome_widths_default.json"), b = load("data/chrome_widths_nohint.json");
report.chromeDefaultVsNohintIdentical = corpus.sizes.every((s) => a[s].every((v, i) => v === b[s][i]));
writeFileSync(here("data/widths_report.json"), JSON.stringify(report, null, 1));
console.log(JSON.stringify(report, null, 1));
