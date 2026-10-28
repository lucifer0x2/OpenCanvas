import assert from "node:assert/strict";
import { connectMcp } from "../../spikes/e2e/protocol.mjs";
import { stacksOperations, FRAME } from "./design.mjs";

export async function author(mcp, slug = "opencanvas-welcome-stacks") {
  const projects = (await mcp.call("list_projects", { limit: 100 })).projects;
  assert.ok(!projects.some((project) => project.slug === slug), "Sample already exists; inspect it instead.");
  const project = await mcp.call("create_project", { slug, name: "OpenCanvas · Mẫu stacks" });
  const operations = stacksOperations();
  const result = await mcp.call("apply_batch", {
    projectId: project.projectId, expectedRevision: 0,
    idempotencyKey: "welcome-stacks-v1", operations,
  });
  assert.equal(result.committedRevision, 1);
  return { ...project, revision: 1, frameId: FRAME, nodeCount: operations.filter((o) => o.op === "create_node").length };
}

if (process.argv[1]?.endsWith("/author.mjs")) {
  const base = process.argv[2];
  assert.ok(base, "Usage: node samples/welcome-stacks/author.mjs http://127.0.0.1:PORT");
  const mcp = await connectMcp(base);
  try { console.log(JSON.stringify(await author(mcp), null, 2)); }
  finally { await mcp.close(); }
}
