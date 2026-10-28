import assert from "node:assert/strict";
import { json, waitFor } from "./protocol.mjs";

export const text =
  "Tiếng Việt — OpenCanvas live MCP. Shared text lines wrap across a measured frame.";
export const token = (value) => ({
  op: "upsert_token",
  token: {
    id: "brand",
    name: "Brand",
    type: "color",
    value,
  },
});
const node = (id, kind, parentId, props) => ({
  op: "create_node",
  id,
  kind,
  parentId,
  props,
});

export async function createDesign(mcp, image) {
  assert.deepEqual((await mcp.call("list_projects", {})).projects, []);
  await mcp.call(
    "create_project",
    { slug: "../escape", name: "Invalid" },
    "INVALID_REQUEST",
  );
  const project = await mcp.call("create_project", {
    slug: "live-e2e",
    name: "Live MCP acceptance",
  });
  const projectId = project.projectId;
  const imported = await mcp.call("import_asset", {
    projectId,
    expectedRevision: 0,
    idempotencyKey: "image-import",
    mediaType: "image/png",
    base64: image,
  });
  const assetId = imported.asset.id;
  const operations = [
    token("#2266cc"),
    {
      op: "upsert_style",
      style: {
        id: "surface",
        name: "Brand surface",
        kind: "fill",
        value: { tokenId: "brand" },
      },
    },
    node("page", "page", null, { name: "Welcome" }),
    node("frame", "frame", "page", {
      name: "Live frame",
      width: 640,
      height: 480,
      fill: "#ffffff",
      layout: { mode: "column", padding: 28, gap: 18 },
    }),
    node("headline", "text", "frame", {
      name: "Headline",
      text,
      width: "fill",
      height: "hug",
      fontId: "font_bundled",
      fontSize: 28,
      lineHeight: 36,
      fill: "#15263b",
    }),
    node("rectangle", "rectangle", "frame", {
      name: "Brand rectangle",
      width: "fill",
      height: 70,
      radius: 12,
      fill: { styleId: "surface" },
    }),
    node("image", "image", "frame", {
      name: "Imported image",
      width: 96,
      height: 64,
      assetId,
      fit: "contain",
    }),
  ];
  const request = {
    projectId,
    expectedRevision: 1,
    idempotencyKey: "create-design",
    operations,
  };
  // Discard the first result to model a caller losing the successful response.
  await mcp.call("apply_batch", request);
  const retry = await mcp.call("apply_batch", request);
  assert.equal(retry.committedRevision, 2);
  assert.equal(retry.replayed, true);
  await mcp.call(
    "apply_batch",
    { ...request, operations: [token("#ffffff")] },
    "IDEMPOTENCY_KEY_REUSED",
  );
  await mcp.call(
    "apply_batch",
    {
      projectId,
      expectedRevision: 2,
      idempotencyKey: "invalid-missing-ref",
      operations: [
        node("must-rollback", "rectangle", "frame", { width: 12, height: 12 }),
        {
          op: "update_node",
          id: "rectangle",
          props: { fill: { tokenId: "missing" } },
        },
      ],
    },
    "VALIDATION_FAILED",
  );
  return { projectId, assetId, request, retry };
}

export async function render(
  mcp,
  base,
  projectId,
  revision,
  width = 640,
  height = 480,
) {
  const pending = await mcp.call("render_frame", {
    projectId,
    revision,
    frameId: "frame",
    viewport: { width, height },
  });
  const job = await waitFor(async () => {
    const result = await json(base, `/api/v1/jobs/${pending.jobId}`);
    assert.notEqual(result.status, "failed", JSON.stringify(result));
    return result.status === "ready" ? result : null;
  }, `strict scene r${revision}`);
  assert.equal(job.error, undefined, JSON.stringify(job));
  assert.equal(job.scene.provenance.revision, revision);
  assert.equal(job.scene.provenance.sourceSceneKey, job.sceneKey);
  assert.equal(job.image.provenance.sourceSceneKey, job.sceneKey);
  assert.equal(
    job.image.provenance.fontManifestHash,
    job.scene.provenance.fontManifestHash,
  );
  assert.equal(
    job.image.provenance.assetManifestHash,
    job.scene.provenance.assetManifestHash,
  );
  assert.ok(job.readiness.every((resource) => resource.status === "ready"));
  // MCP omits the computed scene by default; includeScene:true returns the full record.
  const { scene, ...withoutScene } = job;
  const polled = await mcp.call("render_frame", {
    projectId,
    jobId: job.jobId,
  });
  assert.deepEqual(polled, withoutScene);
  assert.ok(scene);
  const full = await mcp.call("render_frame", {
    projectId,
    jobId: job.jobId,
    includeScene: true,
  });
  assert.deepEqual(full, job);
  return job;
}
