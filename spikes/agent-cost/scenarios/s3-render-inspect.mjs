// S3 — render the stacks frame at 1040/800/390 and inspect each job, polling
// every job at a fixed interval (default 250 ms) the way an agent loop would.
import assert from "node:assert/strict";
import { withEnvironment, seedThenRestart } from "../env.mjs";
import { recorder } from "../meter.mjs";
import { seedStacks } from "../seed.mjs";
import { sleep, summarize } from "./common.mjs";

export const id = "S3";
export const title =
  "render_frame + poll + inspect_layout at 1040×720, 800×720, 390×844";

export const VIEWPORTS = [
  { width: 1040, height: 720 },
  { width: 800, height: 720 },
  { width: 390, height: 844 },
];
const TIMEOUT_MS = 120_000;

/** Reads every inspect page; follows nextCursor when the server provides one. */
async function inspectAll(client, projectId, job, extra) {
  const nodes = [];
  let cursor;
  let pages = 0;
  let truncatedWithoutCursor = false;
  do {
    const read = await client.call("inspect_layout", {
      projectId,
      jobId: job.jobId,
      sceneKey: job.sceneKey,
      ...extra,
      ...(cursor ? { cursor } : {}),
    });
    pages++;
    nodes.push(...read.nodes);
    cursor = read.nextCursor ?? undefined;
    if (read.truncated && !cursor) truncatedWithoutCursor = true;
  } while (cursor);
  return { nodes, pages, truncatedWithoutCursor };
}

export async function run(meter, options) {
  const opts = options.s3;
  const { result, cleanup } = await withEnvironment(async (env) => {
    const project = await seedThenRestart(env, (mcp) => seedStacks(mcp));
    const rec = recorder(meter);
    const client = rec.client(await env.connect());
    const { projectId, frameId, revision } = project;
    // EXTENSION POINT: s3.renderArgs / s3.pollArgs (e.g. {"waitMs":2000}) / s3.inspectArgs.
    rec.phase("enqueue");
    const jobs = [];
    for (const viewport of VIEWPORTS) {
      const t0 = performance.now();
      const job = await client.call("render_frame", {
        projectId,
        frameId,
        revision,
        viewport,
        ...opts.renderArgs,
      });
      jobs.push({ viewport, t0, job, polls: 0, readyMs: null });
    }
    rec.phase("poll");
    const deadline = performance.now() + TIMEOUT_MS;
    const settle = (entry) => {
      assert.notEqual(
        entry.job.status,
        "failed",
        JSON.stringify(entry.job.error),
      );
      if (entry.job.status !== "pending" && entry.readyMs === null)
        entry.readyMs = performance.now() - entry.t0;
    };
    jobs.forEach(settle);
    while (jobs.some((j) => j.job.status === "pending")) {
      assert.ok(
        performance.now() < deadline,
        "Render jobs did not finish in time",
      );
      await sleep(opts.pollIntervalMs);
      for (const entry of jobs.filter((j) => j.job.status === "pending")) {
        entry.job = await client.call("render_frame", {
          projectId,
          jobId: entry.job.jobId,
          ...opts.pollArgs,
        });
        entry.polls++;
        settle(entry);
      }
    }
    rec.phase("inspect");
    const viewports = [];
    for (const entry of jobs) {
      assert.equal(entry.job.status, "ready");
      const inspect = await inspectAll(
        client,
        projectId,
        entry.job,
        opts.inspectArgs,
      );
      viewports.push({
        width: entry.viewport.width,
        height: entry.viewport.height,
        polls: entry.polls,
        readyMs: Math.round(entry.readyMs),
        inspectPages: inspect.pages,
        inspectNodes: inspect.nodes.length,
        uniqueInspectNodes: new Set(inspect.nodes.map((n) => n.id)).size,
        truncatedWithoutCursor: inspect.truncatedWithoutCursor,
      });
    }
    return summarize(id, title, rec, {
      pollIntervalMs: opts.pollIntervalMs,
      pollArgs: opts.pollArgs,
      inspectArgs: opts.inspectArgs,
      totalPolls: viewports.reduce((sum, v) => sum + v.polls, 0),
      viewports,
    });
  });
  return { ...result, cleanup };
}
