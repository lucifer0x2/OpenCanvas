// Part D, Chromium path by stage: one fresh `node stage-run.mjs` per frame × scale, sequential,
// fonts served by an idle release server on a temp data root. Summarizes the cold job (k = 0 of
// a fresh process; startup.scriptStartMs is Node boot to the script's first line) and warm p50/p95 per stage.
import { spawn } from "node:child_process";
import { once } from "node:events";
import { rmSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { FRAMES } from "./frames.mjs";
import { BROWSERS, DROP, LAYOUT_BIN, SCALES, WORK, readJson, settle, startServer, summary, work, writeJson } from "./common.mjs";

const RUN = fileURLToPath(new URL("./stage-run.mjs", import.meta.url));

/** Stage values of one job, flat; `otherMs` is the job time no timer explains. */
export function stagesOf(j) {
  const s = {
    jobMs: j.jobMs, passes: j.passes, measured: j.measured,
    resourcesRttMs: j.resources.rttMs, resourcesPageMs: j.resources.pageMs,
    layoutSpawnMs: j.layout.ms, layoutBytes: j.layout.inBytes + j.layout.outBytes,
    measureRttMs: j.measure.rttMs, measurePageMs: j.measure.pageMs,
    drawRttMs: j.draw.rttMs, drawPageMs: j.draw.drawPageMs, toDataURLPageMs: j.draw.toDataURLPageMs,
    base64Bytes: j.draw.base64Bytes, workerNdjsonMs: j.ndjson.ms,
  };
  s.ipcMs = s.resourcesRttMs - s.resourcesPageMs + s.measureRttMs - s.measurePageMs +
    s.drawRttMs - s.drawPageMs - s.toDataURLPageMs;
  s.otherMs = s.jobMs - s.resourcesRttMs - s.layoutSpawnMs - s.measureRttMs - s.drawRttMs;
  return s;
}

const summarize = (jobs) => {
  const rows = jobs.map(stagesOf);
  return Object.fromEntries(Object.keys(rows[0]).map((k) => [k, summary(rows.map((r) => r[k]))]));
};

async function runOne(f, scale, fontBase) {
  const out = work(`results/stages-${f.name}-s${scale}.json`);
  const png = work(`stage/${f.name}-s${scale}.png`);
  const child = spawn(process.execPath, [RUN, work(`inputs/${f.name}.json`), String(scale), fontBase, out, png], {
    stdio: ["ignore", "inherit", "inherit"],
    env: { ...process.env, OPENCANVAS_LAYOUT_BIN: LAYOUT_BIN, PLAYWRIGHT_BROWSERS_PATH: BROWSERS }, // as scripts/app.mjs
  });
  const [code] = await once(child, "exit");
  if (code !== 0) throw new Error(`stage-run ${f.name} s${scale} exited ${code}`);
  const r = readJson(out);
  return { r };
}

export async function stagesBench() {
  const fontRoot = `${WORK}/data-fonts`;
  const fonts = await startServer(fontRoot);
  const result = {};
  try {
    for (const f of FRAMES)
      for (const scale of SCALES) {
        const { r } = await runOne(f, scale, fonts.base);
        await settle(undefined, [fonts.pid]); // the child's Chromium must be gone before the next run
        const cold = r.jobs[0];
        const key = `${f.name}-s${scale}`;
        const warm = r.jobs.filter((j) => j.k >= DROP);
        result[key] = {
          browserVersion: r.browserVersion, node: r.node,
          startup: r.startup,
          cold: { startupMs: r.startup.readyMs, firstJobMs: cold.jobMs, ...stagesOf(cold) },
          warm: summarize(warm),
          warmNoCache: summarize(r.nocacheJobs.filter((j) => j.k >= DROP)),
        };
        writeJson(work(`scenes/${f.name}-node-s${scale}.json`),
          Object.fromEntries(r.jobs.map((j) => [String(j.width), j.geometry])));
        console.log(key, "cold job", cold.jobMs.toFixed(0), "warm p50", result[key].warm.jobMs.p50.toFixed(1));
      }
  } finally {
    await fonts.stop();
    rmSync(fontRoot, { recursive: true, force: true });
  }
  writeJson(work("results/stages.json"), result);
  return result;
}
