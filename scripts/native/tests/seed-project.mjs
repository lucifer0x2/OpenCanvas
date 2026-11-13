// Disposable test roots only: creates one project with a page, frame and text through MCP,
// so verify-runtime.mjs has a frame to render.
import assert from "node:assert/strict";
import { connectMcp } from "../../../spikes/e2e/protocol.mjs";

const [base] = process.argv.slice(2);
assert.match(base ?? "", /^http:\/\/127\.0\.0\.1:\d+$/, "a loopback base URL is required");
const operations = [
  { op: "create_node", id: "page", kind: "page", parentId: null, props: { name: "Deploy test" } },
  {
    op: "create_node", id: "frame", kind: "frame", parentId: "page",
    props: { name: "Card", width: 320, height: 200, fill: "#ffffff" },
  },
  {
    op: "create_node", id: "text", kind: "text", parentId: "frame",
    props: { text: "Native deploy test", width: 280, height: "hug", fontId: "font_bundled", fontSize: 20, lineHeight: 28 },
  },
];
const mcp = await connectMcp(base);
try {
  const { projectId } = await mcp.call("create_project", { slug: `deploy-test-${Date.now()}`, name: "Deploy test" });
  const committed = await mcp.call("apply_batch", { projectId, expectedRevision: 0, idempotencyKey: "deploy-test-seed", operations });
  assert.equal(committed.committedRevision, 1, JSON.stringify(committed));
  console.log(JSON.stringify({ projectId, revision: committed.committedRevision }));
} finally {
  await mcp.close();
}
