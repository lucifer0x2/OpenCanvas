// Part D report: folds the per-phase results in WORK/results into the small committed
// data/e2e/results.json (environment, frames, per frame × scale numbers, gates) and prints the
// markdown tables of README Phần D. No paths of this machine go into the committed file.
import { execFileSync } from "node:child_process";
import { mkdirSync, writeFileSync } from "node:fs";
import path from "node:path";
import { FRAMES } from "./frames.mjs";
import { REPO, RESULTS, SCALES, readJson, work } from "./common.mjs";
import { tables } from "./tables.mjs";

const sh = (cmd, args, cwd = REPO) => {
  try { return execFileSync(cmd, args, { cwd, encoding: "utf8" }).trim(); } catch { return null; }
};

function environment(stages) {
  const any = Object.values(stages)[0];
  const [cpu, ncpu, mem] = sh("sysctl", ["-n", "machdep.cpu.brand_string", "hw.ncpu", "hw.memsize"]).split("\n");
  const [perf, eff] = (sh("sysctl", ["-n", "hw.perflevel0.physicalcpu", "hw.perflevel1.physicalcpu"]) ?? "").split("\n");
  return {
    date: new Date().toISOString(), model: sh("sysctl", ["-n", "hw.model"]), cpu, ncpu: Number(ncpu),
    cores: perf && eff ? `${perf}P+${eff}E` : null,
    memGiB: Math.round(Number(mem) / 2 ** 30), os: `macOS ${sh("sw_vers", ["-productVersion"])}`,
    power: sh("pmset", ["-g", "batt"])?.split("\n")[0].replace(/^Now drawing from /, "").replace(/'/g, ""),
    node: any.node, chromium: any.browserVersion,
    playwright: readJson(path.join(REPO, "node_modules/playwright/package.json")).version,
    rustc: sh("rustc", ["--version"]), commit: sh("git", ["rev-parse", "--short", "HEAD"]),
    dirty: Boolean(sh("git", ["status", "--porcelain", "--", "spikes/rust-export"])),
  };
}

const pick = (s, keys) => Object.fromEntries(keys.map((k) => [k, s[k]]));
const P50 = (o) => Object.fromEntries(Object.entries(o).map(([k, v]) => [k, v?.p50 ?? v]));

/** The old D8 estimate, recomputed from the full-canvas painter run: paint if text masks covered
 * only node bounds and parents passed the clip mask by reference (linear in area, not measured). */
function paintEstimate(full, canvasPx) {
  const w = P50(full), p = P50(full.profile);
  const fullCanvas = p.rasterMs + p.compositeMs; // coverage + LUT, clip multiply + fill
  const ratio = p.textArea / (p.textNodes * canvasPx);
  const paintMs = w.paintMs - fullCanvas * (1 - ratio) - p.maskMs;
  return { textFullCanvasMs: fullCanvas, maskMs: p.maskMs, bboxAreaRatio: ratio, paintMs,
    totalFastMs: w.parseMs + w.layoutMs + w.measureMs + paintMs + w.encodeFastMs };
}

export function report() {
  const server = readJson(work("results/server.json"));
  const stages = readJson(work("results/stages.json"));
  const rust = readJson(work("results/rust.json"));
  const gate = readJson(work("results/gate.json"));
  const paintGate = readJson(work("results/paint-gate.json"));
  const capture = readJson(work("capture.json"));
  const out = { env: { ...environment(stages), phases: readJson(work("results/phases.json")) }, frames: {}, configs: {},
    gate: {}, paintGate };
  for (const f of FRAMES) {
    const g = gate[f.name];
    out.frames[f.name] = { viewport: f.viewport, ...pick(g, ["sceneNodes", "textNodes", "textLines"]) };
    out.gate[f.name] = { rustVsChrome: g.rustVsChrome, stageVsServer: { ...g.stageVsServer, examples: undefined },
      pixels: g.pixels };
    for (const scale of SCALES) {
      const key = `${f.name}-s${scale}`;
      const sv = server[key], st = stages[key], ru = rust[key];
      const mcp = sv.warm.enqueueMs.p50 + sv.warm.imageMs.p50;
      const canvasPx = Math.ceil(f.viewport.width * scale) * Math.ceil(f.viewport.height * scale);
      // Chromium ÷ Rust, Rust charged the same MCP enqueue + image download the server path measured.
      const vs = (chromium, rustMs) => chromium / (rustMs + mcp);
      out.configs[key] = {
        chromiumServer: { coldMs: sv.cold, coldRuns: sv.coldRuns.map((c) => c.totalMs), warm: sv.warm,
          engineCpu: sv.engineCpu, pngBytes: capture.find((c) => c.frame === f.name && c.scale === scale).pngBytes },
        chromiumStages: { startup: st.startup, cold: st.cold, warm: st.warm, warmNoCache: st.warmNoCache },
        serverNdjson: ru.serverNdjson,
        rust: { threads: ru.threads, cold: ru.cold, cold1: ru.cold1, warm: ru.warm, warm1: ru.warm1, warmFull: ru.warmFull,
          warmNoCache: ru.warmNoCache, textOpts: ru.textOpts, estimate: paintEstimate(ru.warmFull, canvasPx) },
        mcpMs: mcp,
        ratio: {
          cold: vs(sv.cold.p50, ru.cold.wallMs.p50), cold1: vs(sv.cold.p50, ru.cold1.wallMs.p50),
          warm: vs(sv.warm.totalMs.p50, ru.warm.totalMs.p50), warm1: vs(sv.warm.totalMs.p50, ru.warm1.totalMs.p50),
          warmFast: vs(sv.warm.totalMs.p50, ru.warm.totalFastMs.p50), warmFast1: vs(sv.warm.totalMs.p50, ru.warm1.totalFastMs.p50),
          warmFull: vs(sv.warm.totalMs.p50, ru.warmFull.totalMs.p50),
        },
      };
    }
  }
  mkdirSync(RESULTS, { recursive: true });
  // Three decimals; percentile pairs on one line (n is always ITERS - DROP for warm, 3 or 5 for cold).
  const round = (k, v) => (typeof v === "number" && !Number.isInteger(v) ? Math.round(v * 1000) / 1000 : v);
  const text = JSON.stringify(out, round, 1).replace(/\{\s+"p50": ([^,]+),\s+"p95": ([^,]+),\s+"n": (\d+)\s+\}/g, '{"p50": $1, "p95": $2, "n": $3}');
  writeFileSync(path.join(RESULTS, "results.json"), text);
  const md = tables(out);
  console.log(md);
  return out;
}
