// Stable IDs exercise multiple scopes and virtual component children through the real MCP API.
export async function createActionFixture(mcp) {
  const { projectId } = await mcp.call("create_project", {
    slug: "node-actions",
    name: "Node actions",
  });
  const node = (id, kind, parentId, props) => ({
    op: "create_node",
    id,
    kind,
    parentId,
    props,
  });
  await mcp.call("apply_batch", {
    projectId,
    expectedRevision: 0,
    idempotencyKey: "fixture",
    operations: [
      node("decoy-page", "page", null, { name: "Decoy page" }),
      node("decoy", "frame", "decoy-page", {
        name: "Decoy frame",
        width: 300,
        height: 200,
      }),
      node("page", "page", null, { name: "Target page" }),
      node("frame", "frame", "page", {
        name: "Target frame",
        width: 400,
        height: 300,
        fill: "#ffffff",
      }),
      node("box", "rectangle", "frame", {
        name: "Export box",
        x: 30,
        y: 30,
        width: 80,
        height: 50,
        fill: "#2266cc",
      }),
      node("empty", "rectangle", "frame", {
        name: "Empty box",
        x: 30,
        y: 100,
        width: 0,
        height: 30,
        fill: "#2266cc",
      }),
      node("master", "frame", "page", {
        name: "Master",
        component: { name: "Card" },
        width: 100,
        height: 70,
      }),
      node("label", "text", "master", {
        name: "Master label",
        text: "Hello",
        width: 100,
        height: 28,
        fontId: "font_bundled",
        fontSize: 16,
        lineHeight: 22,
      }),
      node("instance", "instance", "frame", {
        name: "Placed card",
        componentId: "master",
        x: 180,
        y: 30,
        overrides: { label: { name: "Placed label", text: "Placed" } },
      }),
    ],
  });
  return projectId;
}
