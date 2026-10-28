import assert from "node:assert/strict";
import { stat, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { json } from "./protocol.mjs";
import { text } from "./design.mjs";

export async function readback({
  mcp,
  base,
  dataRoot,
  projectId,
  assetId,
  artifacts,
}) {
  const snapshot = await json(base, `/api/v1/projects/${projectId}/snapshot`);
  assert.equal(snapshot.revision, 2);
  assert.equal(snapshot.nodes.length, 5);
  assert.ok(!snapshot.nodes.some((node) => node.id === "must-rollback"));
  assert.equal(
    snapshot.nodes.find((node) => node.id === "headline").props.text,
    text,
  );
  assert.equal(
    snapshot.nodes.find((node) => node.id === "image").props.assetId,
    assetId,
  );
  assert.ok((await stat(join(dataRoot, "live-e2e/db.sql"))).isFile());
  assert.ok((await stat(join(dataRoot, "live-e2e/assets"))).isDirectory());
  const outline = await mcp.call("get_outline", { projectId });
  const subtree = await mcp.call("get_subtree", {
    projectId,
    nodeId: "frame",
    depth: 8,
  });
  const context = await mcp.call("get_design_context", {
    projectId,
    frameId: "frame",
  });
  const tokens = await mcp.call("list_tokens", { projectId });
  const styles = await mcp.call("list_styles", { projectId });
  const fonts = await mcp.call("list_fonts", { projectId });
  const assets = await mcp.call("list_assets", { projectId });
  for (const result of [
    outline,
    subtree,
    context,
    tokens,
    styles,
    fonts,
    assets,
  ]) {
    assert.equal(result.revision, 2);
    assert.equal(result.truncated, false);
  }
  assert.deepEqual(
    outline.nodes.map((node) => node.id).sort(),
    snapshot.nodes.map((node) => node.id).sort(),
  );
  assert.deepEqual(
    subtree.nodes.map((node) => node.id),
    ["frame", "headline", "rectangle", "image"],
  );
  const rectangle = context.nodes.find((node) => node.id === "rectangle");
  assert.deepEqual(rectangle.props.fill, { styleId: "surface" });
  assert.equal(rectangle.resolvedProps.fill, "#2266cc");
  assert.equal(
    context.tokens.find((item) => item.id === "brand").resolvedValue,
    "#2266cc",
  );
  assert.equal(
    context.styles.find((item) => item.id === "surface").resolvedValue,
    "#2266cc",
  );
  assert.equal(tokens.items[0].value, "#2266cc");
  assert.deepEqual(styles.items[0].value, { tokenId: "brand" });
  assert.equal(
    fonts.items.find((font) => font.id === "font_bundled").contentHash,
    "bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d",
  );
  assert.equal(assets.items[0].id, assetId);
  assert.equal(assets.items[0].relativeKey, undefined);
  await writeFile(
    new URL("readback.json", artifacts),
    JSON.stringify(
      {
        outline,
        subtree,
        context,
        tokens,
        styles,
        fonts,
        assets,
      },
      null,
      2,
    ),
  );
}

export async function inspect(mcp, job) {
  const result = await mcp.call("inspect_layout", {
    projectId: job.projectId,
    jobId: job.jobId,
    sceneKey: job.sceneKey,
    nodeIds: ["headline", "rectangle", "image"],
  });
  assert.equal(result.revision, job.revision);
  assert.equal(result.sceneKey, job.sceneKey);
  for (const node of result.nodes) {
    const computed = job.scene.nodes.find((entry) => entry.id === node.id);
    assert.deepEqual(node.bounds, computed.bounds);
    assert.deepEqual(node.lines, computed.lines);
  }
  assert.equal(result.nodes.length, 3);
}
