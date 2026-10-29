// Part D shared pieces: paths, the release temp server (port 0, temp data root), leftover-process
// checks, MCP render timing and percentile stats. Nothing here touches port 3847 or ~/.opencanvas.
import { spawn, execFileSync } from "node:child_process";
import { once } from "node:events";
import { mkdirSync, readFileSync, writeFileSync } from "node:fs";
import { tmpdir } from "node:os";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { waitFor } from "../../../e2e/protocol.mjs";

export const REPO = fileURLToPath(new URL("../../../../", import.meta.url));
export const SPIKE = path.join(REPO, "spikes/rust-export");
/** Scratch for data roots, captures, PNGs and per-run JSON; never committed. */
export const WORK = path.resolve(process.env.BENCH_DIR ?? path.join(tmpdir(), "opencanvas-e2e-bench"));
/** Small committed result JSON. */
export const RESULTS = path.join(SPIKE, "data/e2e");
export const SERVER_BIN = path.join(REPO, "target/release/opencanvas-server");
export const LAYOUT_BIN = path.join(REPO, "target/release/layout-step");
export const SPIKE_BIN = path.join(SPIKE, "target/release/rust-spike");
export const BROWSERS = path.join(REPO, ".runtime/browsers");
export const AUTHORED = path.join(WORK, "data-authored");
export const SCALES = [1, 2];
/** Jobs per sequence: k = 0 (cold/first), 1–3 warm-up dropped, 4–24 the warm sample (21 runs). */
export const ITERS = 25;
export const DROP = 4;

export const work = (...p) => {
  const file = path.join(WORK, ...p);
  mkdirSync(path.dirname(file), { recursive: true });
  return file;
};
export const readJson = (file) => JSON.parse(readFileSync(file, "utf8"));
export const writeJson = (file, value) => writeFileSync(file, JSON.stringify(value, null, 1));

/** Release server like `npm start` (scripts/app.mjs): release layout-step, bundled browsers. */
export async function startServer(dataRoot, { pathPrefix, env = {} } = {}) {
  mkdirSync(dataRoot, { recursive: true });
  const child = spawn(SERVER_BIN, ["--data-root", dataRoot, "--port", "0", "--workspace", REPO], {
    cwd: REPO,
    detached: true,
    stdio: ["ignore", "pipe", "pipe"],
    env: {
      ...process.env,
      OPENCANVAS_LAYOUT_BIN: LAYOUT_BIN,
      PLAYWRIGHT_BROWSERS_PATH: BROWSERS,
      PATH: pathPrefix ? `${pathPrefix}:${process.env.PATH}` : process.env.PATH,
      ...env,
    },
  });
  let output = "";
  child.stdout.on("data", (d) => (output += d));
  child.stderr.on("data", (d) => (output += d));
  const base = await waitFor(
    async () => {
      if (child.exitCode !== null) throw new Error(`server exited: ${output}`);
      return output.match(/OpenCanvas (http:\/\/127\.0\.0\.1:\d+)/)?.[1];
    },
    "release server startup",
    15_000,
  );
  return {
    base,
    pid: child.pid,
    async stop() {
      if (child.exitCode === null && child.signalCode === null) {
        const exited = once(child, "exit");
        process.kill(-child.pid, "SIGTERM"); // group: server, Node worker (+ tee wrapper)
        await exited;
      }
      await settle(child.pid);
    },
  };
}

/** Our processes still alive: the server's group, Chromium from this checkout's browsers, a data
 * root, tee wrapper or capture under WORK. Never matches the shell that launched this script. */
export function leftovers(pgid, keep = []) {
  const ps = execFileSync("ps", ["-Ao", "pid=,pgid=,command="], { encoding: "utf8" });
  const ours = [BROWSERS, `${WORK}/data-`, `${WORK}/bin/`, `${WORK}/capture/`];
  return ps
    .split("\n")
    .map((l) => l.trim().match(/^(\d+)\s+(\d+)\s+(.*)$/))
    .filter(Boolean)
    .filter(([, pid, g, cmd]) => ![process.pid, process.ppid, ...keep].includes(Number(pid)) &&
      ((pgid && Number(g) === pgid) || ours.some((s) => cmd.includes(s))))
    .map(([, pid, , cmd]) => ({ pid: Number(pid), cmd: cmd.slice(0, 120) }));
}

/** CPU time so far (user + sys, ms) and thread count of the render engine behind a server: every
 * live descendant of the server process (Node worker, Chromium and its helpers). Short-lived
 * children such as `layout-step` are gone by the time this samples, so they are not counted. */
export function engineCpu(serverPid) {
  const rows = execFileSync("ps", ["-Ao", "pid=,ppid=,time="], { encoding: "utf8" })
    .split("\n").map((l) => l.trim().split(/\s+/)).filter((r) => r.length === 3);
  const kids = new Map();
  for (const [pid, ppid] of rows) kids.set(ppid, [...(kids.get(ppid) ?? []), pid]);
  const ours = [];
  for (let todo = [...(kids.get(String(serverPid)) ?? [])]; todo.length; ) {
    const pid = todo.pop();
    ours.push(pid);
    todo.push(...(kids.get(pid) ?? []));
  }
  const secs = (t) => t.split(":").reduce((s, v) => s * 60 + Number(v), 0); // [hh:]mm:ss.ss
  const cpuMs = rows.filter(([pid]) => ours.includes(pid)).reduce((s, [, , t]) => s + secs(t) * 1000, 0);
  const threads = ours.length
    ? execFileSync("ps", ["-M", "-p", ours.join(",")], { encoding: "utf8" }).trim().split("\n").length - 1 : 0;
  return { cpuMs, processes: ours.length, threads };
}

/** Waits up to 5 s for our processes to exit, then kills and reports what was left. */
export async function settle(pgid, keep = []) {
  const until = Date.now() + 5_000;
  let left = leftovers(pgid, keep);
  while (left.length && Date.now() < until) {
    await new Promise((r) => setTimeout(r, 100));
    left = leftovers(pgid, keep);
  }
  for (const p of left) {
    try { process.kill(p.pid, "SIGKILL"); } catch {}
  }
  if (left.length) console.error("killed leftovers", left);
  return left;
}

/** One user-visible render: enqueue, wait on the job, download the PNG bytes. */
export async function timedRender(mcp, { projectId, revision }, frameId, viewport, scale) {
  const t0 = performance.now();
  const pending = await mcp.call("render_frame", { projectId, frameId, revision, viewport, scale });
  const t1 = performance.now();
  let job = pending;
  while (job.status === "pending")
    job = await mcp.call("render_frame", { projectId, jobId: pending.jobId, waitMs: 10_000 });
  const t2 = performance.now();
  if (job.status !== "ready" || !job.imageUrl) throw new Error(`render failed: ${JSON.stringify(job.error)}`);
  const png = Buffer.from(await (await fetch(job.imageUrl)).arrayBuffer());
  const t3 = performance.now();
  return { jobId: job.jobId, imageUrl: job.imageUrl, png, wasPending: pending.status === "pending",
    totalMs: t3 - t0, enqueueMs: t1 - t0, waitMs: t2 - t1, imageMs: t3 - t2 };
}

/** Nearest-rank percentile. */
export function pct(values, q) {
  const v = values.filter((x) => Number.isFinite(x)).sort((a, b) => a - b);
  return v.length ? v[Math.max(0, Math.ceil(q * v.length) - 1)] : null;
}
export const summary = (values) => ({ p50: pct(values, 0.5), p95: pct(values, 0.95), n: values.length });
export const r2 = (v) => (v == null ? v : Math.round(v * 100) / 100);
