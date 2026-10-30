// Part A.5: line lists from the Rust wrap port vs Chrome's real measureBatch.
import { readFileSync, writeFileSync } from "node:fs";
const here = (p) => new URL(`../${p}`, import.meta.url);
const load = (p) => JSON.parse(readFileSync(here(p)));
const out = {};
for (const mode of ["default", "nohint"]) for (const rf of ["rust_wrap.json", "rust_wrap_v2.json"]) {
  const chrome = load(`data/chrome_wrap_${mode}.json`), rust = load(`data/${rf}`);
  let sameText = 0, sameAll = 0; const diffs = []; let maxLineWidthDelta = 0, ydiff = 0;
  for (const r of rust) {
    const c = chrome[`${r.p}|${r.size}|${r.width}`];
    const tc = c.lines.map((l) => l.text), tr = r.lines.map((l) => l.text);
    const textEq = JSON.stringify(tc) === JSON.stringify(tr);
    if (textEq) {
      sameText++;
      const wEq = c.lines.every((l, i) => l.width === r.lines[i].width && l.y === r.lines[i].y);
      if (wEq) sameAll++;
      c.lines.forEach((l, i) => { maxLineWidthDelta = Math.max(maxLineWidthDelta, Math.abs(l.width - r.lines[i].width)); if (l.y !== r.lines[i].y) ydiff++; });
    } else diffs.push({ p: r.p, size: r.size, width: r.width, chrome: tc, rust: tr });
  }
  out[`${mode}/${rf}`] = { cases: rust.length, identicalLineText: sameText, identicalTextWidthY: sameAll, maxLineWidthDelta, ydiff, diffs };
}
writeFileSync(here("data/wrap_report.json"), JSON.stringify(out, null, 1));
for (const [k, v] of Object.entries(out)) console.log(k, JSON.stringify({ ...v, diffs: v.diffs.length }), v.diffs.slice(0, 3));
