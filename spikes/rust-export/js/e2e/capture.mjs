// Part D capture: one server run whose `node` (the server spawns `node worker/cli.mjs` from PATH,
// crates/server/src/worker.rs) is a scratch wrapper that tees the worker's NDJSON stdin/stdout.
// Yields the exact layout input the server sends, the Chrome PNG per frame × scale and the
// server's scene geometry. Product code is untouched; timing runs never use the wrapper.
import { chmodSync, readFileSync, rmSync, writeFileSync } from "node:fs";
import { connectMcp } from "../../../e2e/protocol.mjs";
import { FRAMES } from "./frames.mjs";
import { geometry } from "./geometry.mjs";
import { AUTHORED, SCALES, readJson, startServer, timedRender, work, writeJson } from "./common.mjs";

function wrapper() {
  const dir = work("bin/node");
  const tee = work("capture/x").replace(/\/x$/, "");
  writeFileSync(dir, `#!/bin/sh
if [ "$1" = "worker/cli.mjs" ]; then
  tee -a "${tee}/in.ndjson" | "${process.execPath}" "$@" | tee -a "${tee}/out.ndjson"
  exit 0
fi
exec "${process.execPath}" "$@"
`);
  chmodSync(dir, 0o755);
  rmSync(`${tee}/in.ndjson`, { force: true });
  rmSync(`${tee}/out.ndjson`, { force: true });
  return { bin: dir.replace(/\/node$/, ""), tee };
}

export async function capture() {
  const projects = readJson(work("projects.json"));
  const { bin, tee } = wrapper();
  const server = await startServer(AUTHORED, { pathPrefix: bin });
  const mcp = await connectMcp(server.base);
  const jobs = [];
  try {
    for (const f of FRAMES)
      for (const scale of SCALES) {
        const r = await timedRender(mcp, projects[f.slug], f.frameId, f.viewport, scale);
        writeFileSync(work(`chrome/${f.name}-s${scale}.png`), r.png);
        const job = await (await fetch(`${server.base}/api/v1/jobs/${r.jobId}`)).json();
        writeJson(work(`scenes/${f.name}-server.json`), geometry(job.scene));
        jobs.push({ frame: f.name, scale, pngBytes: r.png.length, nodes: job.scene.nodes.length,
          textNodes: job.scene.nodes.filter((n) => n.kind === "text").length,
          lines: job.scene.nodes.reduce((s, n) => s + n.lines.length, 0) });
      }
  } finally {
    await mcp.close();
    await server.stop();
  }
  // Requests are compute, render per job in the order rendered above; keep each frame's input.
  const requests = readFileSync(`${tee}/in.ndjson`, "utf8").trim().split("\n").map((l) => JSON.parse(l));
  const computes = requests.filter((r) => r.op === "compute");
  if (computes.length !== jobs.length) throw new Error(`captured ${computes.length} computes for ${jobs.length} jobs`);
  FRAMES.forEach((f, i) => writeJson(work(`inputs/${f.name}.json`), computes[i * SCALES.length].input));
  writeJson(work("capture.json"), jobs);
  return jobs;
}
