import assert from "node:assert/strict";
import { mkdir, mkdtemp, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { startServer } from "../../spikes/e2e/server.mjs";
import { connectMcp, waitFor, json } from "../../spikes/e2e/protocol.mjs";
import { welcomeOperations, FRAME as OLD_FRAME } from "../welcome/design.mjs";
import { author } from "./author.mjs";
import { compareImages } from "./pixels.mjs";
import { FRAME, DESKTOP, NARROW, THRESHOLD } from "./design.mjs";

const artifacts = new URL("../../artifacts/sample-stacks/", import.meta.url);
const selected = ["sidebar", "body", "body-content", "stats", "completed", "active", "focus", "weekly-chart", "weekly-goal", "today-tasks"];
await mkdir(artifacts, { recursive: true });
await writeFile(new URL("threshold.json", artifacts), JSON.stringify(THRESHOLD, null, 2));
console.log("Predeclared desktop comparison threshold", THRESHOLD);
const root = await mkdtemp(join(tmpdir(), "opencanvas-stacks-"));
let server, mcp;
const summary = { temporaryData: true, threshold: THRESHOLD, viewports: [], success: false, cleanup: false };
try {
  server = await startServer(root);
  mcp = await connectMcp(server.base);
  const project = await author(mcp);
  const baseline = await mcp.call("create_project", { slug: "baseline", name: "Temporary baseline" });
  await mcp.call("apply_batch", { projectId: baseline.projectId, expectedRevision: 0, idempotencyKey: "baseline-v1", operations: welcomeOperations() });
  const snapshot = await json(server.base, `/api/v1/projects/${project.projectId}/snapshot`);
  assert.equal(snapshot.nodes.length, project.nodeCount);
  await writeFile(new URL("authored-snapshot.json", artifacts), JSON.stringify(snapshot, null, 2));
  const contexts = [];
  let cursor;
  do {
    const context = await mcp.call("get_design_context", {
      projectId: project.projectId, frameId: FRAME, revision: 1, viewport: DESKTOP,
      depth: 8, limit: 200, budget: 65536, ...(cursor ? { cursor } : {}),
    });
    contexts.push(context);
    cursor = context.nextCursor;
  } while (cursor);
  assert.equal(contexts.flatMap((c) => c.nodes).length, project.nodeCount - 1);
  await writeFile(new URL("context-pages.json", artifacts), JSON.stringify(contexts, null, 2));
  summary.authored = { nodes: snapshot.nodes.length,
    layoutModes: snapshot.nodes.reduce((counts, n) => {
      if (n.props.layout) counts[n.props.layout.mode] = (counts[n.props.layout.mode] ?? 0) + 1;
      return counts;
    }, {}) };
  async function render(projectId, frameId, viewport, prefix) {
    const pending = await mcp.call("render_frame", { projectId, frameId, revision: 1, viewport });
    const job = await waitFor(async () => {
      const value = await mcp.call("render_frame", { projectId, jobId: pending.jobId });
      assert.notEqual(value.status, "failed", JSON.stringify(value.error));
      return value.status === "ready" ? value : null;
    }, prefix);
    assert.ok(job.readiness.every((r) => r.status === "ready"));
    const inspect = await mcp.call("inspect_layout", { projectId, jobId: job.jobId, sceneKey: job.sceneKey });
    assert.equal(inspect.truncated, false);
    const response = await fetch(new URL(job.imageUrl, server.base));
    assert.equal(response.status, 200);
    await writeFile(new URL(`${prefix}.png`, artifacts), Buffer.from(await response.arrayBuffer()));
    await writeFile(new URL(`${prefix}-job.json`, artifacts), JSON.stringify(job, null, 2));
    await writeFile(new URL(`${prefix}-inspect.json`, artifacts), JSON.stringify(inspect, null, 2));
    return { viewport, sceneKey: job.sceneKey,
      bounds: Object.fromEntries(inspect.nodes.filter((n) => selected.includes(n.id)).map((n) => [n.id, n.bounds])) };
  }
  await render(baseline.projectId, OLD_FRAME, DESKTOP, "baseline-1040");
  for (const viewport of [DESKTOP, { width: 800, height: 720 }, NARROW]) {
    summary.viewports.push(await render(project.projectId, FRAME, viewport, `stacks-${viewport.width}`));
  }
  const [desktop, medium, narrow] = summary.viewports.map((v) => v.bounds);
  assert.equal(desktop.sidebar.width, 176);
  assert.equal(medium.sidebar.width, 176);
  assert.equal(narrow.sidebar.width, 176);
  assert.ok(desktop.body.width > medium.body.width && medium.body.width > narrow.body.width);
  assert.equal(narrow.body.width, 480);
  assert.equal(narrow.completed.width, 120);
  assert.ok(desktop.completed.width > medium.completed.width);
  summary.comparison = await compareImages(artifacts);
  summary.mcpCalls = mcp.calls;
  summary.success = true;
} finally {
  if (mcp) await mcp.close();
  if (server) await server.stop();
  await rm(root, { recursive: true, force: true });
  summary.cleanup = true;
  await writeFile(new URL("verification.json", artifacts), JSON.stringify(summary, null, 2));
}
console.log(JSON.stringify(summary, null, 2));
