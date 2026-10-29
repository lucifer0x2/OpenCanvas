// Part D runner: node spikes/rust-export/js/e2e/run.mjs [phase ...]
// Phases, in order: author capture server stages paintgate rust gate report cleanup (default: all).
// Scratch goes to $BENCH_DIR (default $TMPDIR/opencanvas-e2e-bench); `cleanup` deletes the authored
// data root. After every phase no server, worker or Chromium of ours may be left running.
import { existsSync, rmSync } from "node:fs";
import { loadavg } from "node:os";
import { authorFrames } from "./frames.mjs";
import { capture } from "./capture.mjs";
import { serverBench } from "./server.mjs";
import { stagesBench } from "./stages.mjs";
import { paintGate } from "./paint-gate.mjs";
import { rustBench } from "./rust.mjs";
import { gate } from "./gate.mjs";
import { report } from "./report.mjs";
import { AUTHORED, WORK, leftovers, readJson, settle, work, writeJson } from "./common.mjs";

const cleanup = () => rmSync(AUTHORED, { recursive: true, force: true });
const PHASES = { author: authorFrames, capture, server: serverBench, stages: stagesBench, paintgate: paintGate,
  rust: rustBench, gate, report, cleanup };
const wanted = process.argv.slice(2).length ? process.argv.slice(2) : Object.keys(PHASES);
console.log("scratch", WORK);
try {
  for (const name of wanted) {
    if (!PHASES[name]) throw new Error(`unknown phase ${name}`);
    const t = performance.now();
    const log = existsSync(work("results/phases.json")) ? readJson(work("results/phases.json")) : {};
    log[name] = { start: new Date().toISOString(), load1mBefore: loadavg()[0] };
    await PHASES[name]();
    const left = await settle();
    Object.assign(log[name], { seconds: (performance.now() - t) / 1000, load1mAfter: loadavg()[0] });
    if (name !== "cleanup") writeJson(work("results/phases.json"), log);
    console.log(`phase ${name} done in ${((performance.now() - t) / 1000).toFixed(1)} s; leftovers killed: ${left.length}`);
  }
} finally {
  const left = leftovers();
  console.log(left.length ? `LEFTOVER PROCESSES: ${JSON.stringify(left)}` : "no leftover processes");
}
