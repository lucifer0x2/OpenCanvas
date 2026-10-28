import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { connectMcp, json, waitFor } from "../../spikes/e2e/protocol.mjs";
import { FRAME, VIEWPORT, welcomeOperations } from "./design.mjs";

const base = "http://127.0.0.1:3847";
const artifacts = new URL("../../artifacts/sample/", import.meta.url);
const mcp = await connectMcp(base);
try {
  const projects = (await mcp.call("list_projects", { limit: 100 })).projects;
  assert.ok(!projects.some((project) => project.slug === "opencanvas-welcome"),
    "The sample already exists; inspect it instead of creating a duplicate.");
  const project = await mcp.call("create_project", {
    slug: "opencanvas-welcome", name: "OpenCanvas · Mẫu đầu tiên",
  });
  const operations = welcomeOperations();
  const batch = {
    projectId: project.projectId, expectedRevision: 0,
    idempotencyKey: "welcome-design-v1", operations,
  };
  const commit = await mcp.call("apply_batch", batch);
  assert.equal(commit.committedRevision, 1);
  const snapshot = await json(base, `/api/v1/projects/${project.projectId}/snapshot`);
  assert.equal(snapshot.revision, 1);
  assert.equal(snapshot.nodes.length, operations.filter((op) => op.op === "create_node").length);
  const pending = await mcp.call("render_frame", {
    projectId: project.projectId, revision: 1, frameId: FRAME, viewport: VIEWPORT,
  });
  const job = await waitFor(async () => {
    const value = await mcp.call("render_frame", { projectId: project.projectId, jobId: pending.jobId });
    assert.notEqual(value.status, "failed", JSON.stringify(value.error));
    return value.status === "ready" ? value : null;
  }, "welcome PNG");
  assert.equal(job.scene.provenance.revision, 1);
  assert.equal(job.scene.provenance.sourceSceneKey, job.sceneKey);
  assert.equal(job.image.provenance.sourceSceneKey, job.sceneKey);
  assert.ok(job.readiness.every((resource) => resource.status === "ready"));
  const context = await mcp.call("get_design_context", {
    projectId: project.projectId, frameId: FRAME, revision: 1, depth: 8, limit: 200, budget: 65536,
  });
  const response = await fetch(new URL(job.imageUrl, base));
  assert.equal(response.status, 200);
  assert.match(response.headers.get("content-type"), /image\/png/);
  await mkdir(artifacts, { recursive: true });
  await writeFile(new URL("dashboard.png", artifacts), Buffer.from(await response.arrayBuffer()));
  await writeFile(new URL("job.json", artifacts), JSON.stringify(job, null, 2));
  await writeFile(new URL("context.json", artifacts), JSON.stringify(context, null, 2));
  const summary = {
    ...project, revision: 1, frameId: FRAME, viewport: VIEWPORT,
    nodeCount: snapshot.nodes.length, tokenCount: snapshot.tokens.length,
    styleCount: snapshot.styles.length, jobId: job.jobId, sceneKey: job.sceneKey,
    viewerUrl: base, imageUrl: new URL(job.imageUrl, base).href,
    createdThrough: "MCP create_project + one atomic apply_batch at revision 0",
  };
  await writeFile(new URL("summary.json", artifacts), JSON.stringify(summary, null, 2));
  console.log(JSON.stringify(summary, null, 2));
} finally {
  await mcp.close();
}
