// Part D, Chromium path at server level (what an MCP client sees): render_frame enqueue →
// render_frame {jobId, waitMs} → GET imageUrl bytes. Cold = first job after a fresh server start
// (worker, Playwright and Chromium start lazily inside it), repeated COLD_REPS times with a
// restart each. Warm = jobs 1..ITERS-1 after the cold one, viewport width +k px so every job has
// a new scene key; the worker's in-memory measure cache persists, exactly as in production.
// Around the warm jobs, the CPU time of the worker + Chromium processes is sampled (engineCpu).
import { connectMcp } from "../../../e2e/protocol.mjs";
import { FRAMES } from "./frames.mjs";
import { AUTHORED, DROP, ITERS, SCALES, engineCpu, readJson, startServer, summary, timedRender, work, writeJson } from "./common.mjs";

const COLD_REPS = 3;
const strip = ({ png, imageUrl, jobId, ...t }) => ({ ...t, pngBytes: png.length });

async function session(fn) {
  const server = await startServer(AUTHORED);
  const mcp = await connectMcp(server.base);
  try {
    return await fn(mcp, server);
  } finally {
    await mcp.close();
    await server.stop();
  }
}

export async function serverBench() {
  const projects = readJson(work("projects.json"));
  const out = {};
  for (const f of FRAMES)
    for (const scale of SCALES) {
      const project = projects[f.slug];
      const runs = [];
      const cold = [];
      let cpu;
      for (let rep = 0; rep < COLD_REPS; rep++)
        await session(async (mcp, server) => {
          cold.push(strip(await timedRender(mcp, project, f.frameId, f.viewport, scale)));
          if (rep > 0) return;
          const before = engineCpu(server.pid);
          for (let k = 1; k < ITERS; k++) {
            const viewport = { width: f.viewport.width + k, height: f.viewport.height };
            runs.push({ k, ...strip(await timedRender(mcp, project, f.frameId, viewport, scale)) });
          }
          const after = engineCpu(server.pid);
          cpu = { perJobMs: (after.cpuMs - before.cpuMs) / (ITERS - 1), processes: after.processes, threads: after.threads };
        });
      const warm = runs.filter((r) => r.k >= DROP);
      const key = `${f.name}-s${scale}`;
      out[key] = {
        cold: summary(cold.map((c) => c.totalMs)), coldRuns: cold,
        warm: Object.fromEntries(["totalMs", "enqueueMs", "waitMs", "imageMs"].map((m) => [m, summary(warm.map((r) => r[m]))])),
        engineCpu: cpu, pngBytes: cold[0].pngBytes, runs,
      };
      console.log(key, "cold", out[key].cold.p50?.toFixed(0), "warm p50", out[key].warm.totalMs.p50?.toFixed(1),
        "engine CPU/job", cpu.perJobMs.toFixed(1), `(${cpu.processes} processes, ${cpu.threads} threads)`);
    }
  writeJson(work("results/server.json"), out);
  return out;
}
