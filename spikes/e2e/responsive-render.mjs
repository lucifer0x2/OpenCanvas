import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { writeFile } from "node:fs/promises";
import { waitFor } from "./protocol.mjs";
import { descendants } from "./bounds.mjs";

export async function renderSample(mcp, base, sample, viewport, artifacts) {
  const pending = await mcp.call("render_frame", {
    projectId: sample.projectId,
    frameId: sample.frameId,
    revision: 1,
    viewport,
  });
  const job = await waitFor(async () => {
    const result = await mcp.call("render_frame", {
      projectId: sample.projectId,
      jobId: pending.jobId,
      includeScene: true,
    });
    assert.notEqual(result.status, "failed", JSON.stringify(result));
    return result.status === "ready" ? result : null;
  }, `${sample.kind} ${viewport.width}`);
  assert.equal(job.revision, 1);
  assert.deepEqual(job.scene.provenance.viewport, viewport);
  for (const provenance of [
    job.provenance,
    job.scene.provenance,
    job.image.provenance,
  ]) {
    assert.equal(provenance.sourceSceneKey, job.sceneKey);
    assert.equal(provenance.fontManifestHash, job.provenance.fontManifestHash);
    assert.equal(
      provenance.assetManifestHash,
      job.provenance.assetManifestHash,
    );
  }
  assert.ok(
    job.readiness.length > 0 &&
      job.readiness.every((r) => r.status === "ready"),
  );
  const inspection = await mcp.call("inspect_layout", {
    projectId: sample.projectId,
    jobId: job.jobId,
    sceneKey: job.sceneKey,
  });
  assert.equal(inspection.truncated, false);
  assert.equal(inspection.nodes.length, sample.nodeCount);
  assert.deepEqual(
    inspection.nodes.map((n) => ({ id: n.id, bounds: n.bounds })),
    job.scene.nodes.map((n) => ({ id: n.id, bounds: n.bounds })),
  );
  const bounds = descendants(inspection.nodes, sample.frameId);
  const response = await fetch(new URL(job.imageUrl, base));
  assert.equal(response.status, 200);
  assert.equal(response.headers.get("content-type"), "image/png");
  const png = Buffer.from(await response.arrayBuffer());
  assert.equal(png.readUInt32BE(16), viewport.width);
  assert.equal(png.readUInt32BE(20), viewport.height);
  const prefix = `${sample.kind}-${viewport.width}`;
  if (artifacts) {
    await writeFile(new URL(`${prefix}.png`, artifacts), png);
    await writeFile(
      new URL(`${prefix}-job.json`, artifacts),
      JSON.stringify(job, null, 2),
    );
  }
  return {
    viewport,
    bounds,
    jobId: job.jobId,
    sceneKey: job.sceneKey,
    provenance: job.provenance,
    imageProvenance: job.image.provenance,
    png: {
      bytes: png.length,
      sha256: createHash("sha256").update(png).digest("hex"),
    },
    pngData: png.toString("base64"),
  };
}

export async function comparePng(page, before, after, viewport) {
  const pixels = await page.evaluate(
    async ({ before, after, viewport }) => {
      async function decode(base64) {
        const image = new Image();
        image.src = `data:image/png;base64,${base64}`;
        await image.decode();
        if (image.width !== viewport.width || image.height !== viewport.height)
          throw Error("PNG dimensions");
        const canvas = document.createElement("canvas");
        canvas.width = image.width;
        canvas.height = image.height;
        const ctx = canvas.getContext("2d");
        ctx.drawImage(image, 0, 0);
        return ctx.getImageData(0, 0, image.width, image.height).data;
      }
      const a = await decode(before),
        b = await decode(after);
      let differences = 0;
      for (let i = 0; i < a.length; i++) if (a[i] !== b[i]) differences++;
      return {
        ...viewport,
        differences,
        comparedChannels: a.length,
        tolerance: 0,
      };
    },
    { before, after, viewport },
  );
  assert.equal(
    pixels.differences,
    0,
    "independent post-restart PNG RGBA equality",
  );
  return pixels;
}
