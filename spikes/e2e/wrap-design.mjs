import assert from "node:assert/strict";
import { responsiveFixture } from "../pipeline/responsive-fixture.mjs";
import { assertResponsive } from "../pipeline/responsive-assert.mjs";
import { waitFor } from "./protocol.mjs";

export const wrapViewports = [1040, 640, 390].map((width) => ({
  width,
  height: 844,
}));
export function wrapOperations() {
  const input = responsiveFixture(1040, { text: true });
  return [
    { op: "create_node", id: "page", parentId: null, kind: "page", props: {} },
    ...input.nodes.map((node) => ({
      op: "create_node",
      ...node,
      parentId: node.parentId ?? "page",
    })),
  ];
}
export async function wrapContext(mcp, projectId) {
  const args = { projectId, frameId: "frame", viewports: wrapViewports };
  const context = await waitFor(async () => {
    const value = await mcp.call("get_design_context", args);
    assert.notEqual(value.computed.status, "failed", JSON.stringify(value));
    return value.computed.status === "ready" ? value : null;
  }, "wrap context");
  assert.equal(context.revision, 1);
  assert.equal(context.reflow, true);
  const row = context.nodes.find((n) => n.id === "cards");
  assert.equal(row.props.layout.wrap, true);
  assert.deepEqual(
    row.computedBounds.map((b) => b.height),
    [128, 224, 320],
  );
  return context;
}
export function checkWrapJob(job, index) {
  assertResponsive(job.scene, index + 1, [128, 224, 320][index], true);
}
