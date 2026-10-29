// Part D, Rust in-process prototype (src/bench.rs): cold = whole process from spawn to exit with the
// PNG written (input read, font read + SHA-256 + parse, thread pool, layout, measure, paint, encode),
// COLD_REPS times; warm = one process running the same viewport sequence as the server (+1 px per
// job) with a persistent measure cache. Painter configurations: bbox at N threads (one per core,
// the default) and at 1 thread, plus the original full-canvas painter at 1 thread (the "before"
// numbers and the old D8 estimate). A fresh-cache run per job feeds D6. Also replays the captured
// NDJSON through `rust-spike ndjson` to time the server's side of the worker protocol.
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { FRAMES } from "./frames.mjs";
import { DROP, ITERS, SCALES, SPIKE_BIN, readJson, summary, work, writeJson } from "./common.mjs";

const COLD_REPS = 5;
const STAGES = ["totalMs", "totalFastMs", "cpuMs", "parseMs", "layoutMs", "passes", "measureMs", "measured",
  "paintMs", "encodeMs", "encodeFastMs", "pngBytes", "pngFastBytes"];
const PROFILE = ["threads", "textNodes", "textArea", "maskArea", "fallbacks", "pathMs", "rasterMs", "compositeMs",
  "shapeMs", "masks", "maskMs"];
const N = ["--threads", "0"], ONE = ["--threads", "1"];

function bench(input, scale, extra) {
  const json = work(`rust/${extra.tag}.json`);
  const args = ["bench", input, "--scale", String(scale), "--json", json, ...extra.args];
  const t = performance.now();
  const r = spawnSync(SPIKE_BIN, args, { stdio: ["ignore", "inherit", "inherit"] });
  const wallMs = performance.now() - t;
  if (r.status !== 0) throw new Error(`rust-spike ${args.join(" ")} failed`);
  return { ...readJson(json), wallMs };
}

const summarize = (runs) => ({
  ...Object.fromEntries(STAGES.map((k) => [k, summary(runs.map((r) => r[k]))])),
  profile: Object.fromEntries(PROFILE.map((k) => [k, summary(runs.map((r) => r.profile[k]))])),
});

const coldOf = (cold) => {
  const median = [...cold].sort((a, b) => a.wallMs - b.wallMs)[Math.floor(cold.length / 2)];
  return { wallMs: summary(cold.map((c) => c.wallMs)), wallRuns: cold.map((c) => c.wallMs),
    firstRunDoneMs: summary(cold.map((c) => c.firstRunDoneMs)), threads: median.threads,
    startup: median.startup, run: median.runs[0] }; // from the median-wall process
};

export async function rustBench() {
  const out = {};
  for (const f of FRAMES)
    for (const scale of SCALES) {
      const key = `${f.name}-s${scale}`;
      const input = work(`inputs/${f.name}.json`);
      const [coldN, cold1] = [[], []];
      for (let i = 0; i < COLD_REPS; i++) { // interleaved so drift hits both alike
        coldN.push(bench(input, scale, { tag: `${key}-cold-${i}`, args: [...N, "--iters", "1", "--png", work(`rust/${key}.png`)] }));
        cold1.push(bench(input, scale, { tag: `${key}-cold1-${i}`, args: [...ONE, "--iters", "1"] }));
      }
      const warm = bench(input, scale, { tag: `${key}-warm`, args: [...N, "--iters", String(ITERS),
        ...(scale === 1 ? ["--geometry", work(`scenes/${f.name}-rust.json`)] : [])] });
      const warm1 = bench(input, scale, { tag: `${key}-warm1`, args: [...ONE, "--iters", String(ITERS)] });
      const full = bench(input, scale, { tag: `${key}-full`, args: [...ONE, "--paint", "full", "--iters", String(ITERS)] });
      const nocache = bench(input, scale, { tag: `${key}-nocache`, args: [...N, "--iters", String(ITERS), "--nocache"] });
      out[key] = {
        cold: coldOf(coldN), cold1: coldOf(cold1),
        warm: summarize(warm.runs.slice(DROP)), warm1: summarize(warm1.runs.slice(DROP)),
        warmFull: summarize(full.runs.slice(DROP)), warmNoCache: summarize(nocache.runs.slice(DROP)),
        threads: warm.threads, textOpts: warm.textOpts,
      };
      const w = out[key];
      console.log(key, `threads ${w.threads}`, "cold p50 N/1", w.cold.wallMs.p50.toFixed(1), w.cold1.wallMs.p50.toFixed(1),
        "warm p50 N/1/full", w.warm.totalMs.p50.toFixed(2), w.warm1.totalMs.p50.toFixed(2), w.warmFull.totalMs.p50.toFixed(1));
    }
  // Server-side NDJSON handling, on the lines captured in capture.mjs (FRAMES × SCALES order).
  const tee = work("capture/x").replace(/\/x$/, "");
  if (existsSync(`${tee}/in.ndjson`)) {
    const jobs = JSON.parse(execFileSync(SPIKE_BIN, ["ndjson", `${tee}/in.ndjson`, `${tee}/out.ndjson`, "20"],
      { encoding: "utf8", maxBuffer: 1 << 26 }));
    let i = 0;
    for (const f of FRAMES) for (const scale of SCALES) out[`${f.name}-s${scale}`].serverNdjson = jobs[i++];
  }
  writeJson(work("results/rust.json"), out);
  return out;
}
