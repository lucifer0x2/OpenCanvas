// Part D correctness gate before any timing comparison: Rust scene geometry vs Chromium scene
// geometry for the same layout inputs (every node's bounds, every text line's text/x/y/width,
// at all ITERS viewport widths), the stage script vs the server at the base width, then
// whole-frame pixel diff of the Rust PNG against the server's Chrome PNG.
import { execFileSync } from "node:child_process";
import { readFileSync, statSync } from "node:fs";
import { FRAMES } from "./frames.mjs";
import { SCALES, SPIKE_BIN, readJson, work, writeJson } from "./common.mjs";

const q64 = (v) => Math.round(v * 64) / 64;

/** Compares two geometry dumps of one scene; `exact` false compares after 1/64 quantization. */
export function compareGeometry(a, b, exact = true) {
  const eq = (x, y) => (exact ? x === y : q64(x) === q64(y));
  const byId = new Map(b.map((n) => [n[0], n]));
  const r = { nodes: a.length, nodeMismatches: 0, lines: 0, lineMismatches: 0, missing: 0, examples: [] };
  if (a.length !== b.length) r.missing += Math.abs(a.length - b.length);
  for (const n of a) {
    const m = byId.get(n[0]);
    if (!m) { r.missing++; continue; }
    if (![1, 2, 3, 4].every((i) => eq(n[i], m[i]))) {
      r.nodeMismatches++;
      if (r.examples.length < 5) r.examples.push({ id: n[0], a: n.slice(1, 5), b: m.slice(1, 5) });
    }
    r.lines += Math.max(n[5].length, m[5].length);
    for (let i = 0; i < Math.max(n[5].length, m[5].length); i++) {
      const [la, lb] = [n[5][i], m[5][i]];
      if (!la || !lb || la[0] !== lb[0] || ![1, 2, 3].every((j) => eq(la[j], lb[j]))) {
        r.lineMismatches++;
        if (r.examples.length < 5) r.examples.push({ id: n[0], line: i, a: la, b: lb });
      }
    }
  }
  return r;
}

function add(total, r) {
  for (const k of ["nodes", "nodeMismatches", "lines", "lineMismatches", "missing"]) total[k] = (total[k] ?? 0) + r[k];
  total.examples = [...(total.examples ?? []), ...r.examples].slice(0, 5);
  return total;
}

export function gate() {
  const empty = work("regions-empty.json");
  writeJson(empty, { regions: [] });
  const out = {};
  for (const f of FRAMES) {
    const rust = readJson(work(`scenes/${f.name}-rust.json`));
    const node = readJson(work(`scenes/${f.name}-node-s1.json`));
    const server = readJson(work(`scenes/${f.name}-server.json`));
    const widths = Object.keys(rust);
    const total = { widths: widths.length };
    for (const w of widths) add(total, compareGeometry(rust[w], node[w] ?? []));
    const nodes = rust[widths[0]];
    out[f.name] = {
      sceneNodes: nodes.length, textNodes: nodes.filter((n) => n[5].length).length,
      textLines: nodes.reduce((s, n) => s + n[5].length, 0),
      rustVsChrome: total,
      stageVsServer: compareGeometry(node[String(f.viewport.width)], server),
      pixels: {},
    };
    for (const scale of SCALES) {
      const chrome = work(`chrome/${f.name}-s${scale}.png`);
      const rustPng = work(`rust/${f.name}-s${scale}.png`);
      const stats = JSON.parse(execFileSync(SPIKE_BIN, ["compare", chrome, rustPng, empty, String(scale)], { encoding: "utf8" }));
      const stagePng = work(`stage/${f.name}-s${scale}.png`);
      out[f.name].pixels[`s${scale}`] = {
        ...stats.all,
        chromePngBytes: statSync(chrome).size, rustPngBytes: statSync(rustPng).size,
        rustFastPngBytes: statSync(rustPng.replace(".png", "-fast.png")).size,
        stagePngIdenticalToServer: readFileSync(stagePng).equals(readFileSync(chrome)),
      };
    }
    console.log(f.name, "geometry", JSON.stringify({ ...total, examples: undefined }));
  }
  writeJson(work("results/gate.json"), out);
  return out;
}
