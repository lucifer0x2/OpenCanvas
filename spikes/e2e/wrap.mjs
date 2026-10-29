import assert from "node:assert/strict";
import { writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import {
  wrapOperations,
  wrapContext,
  wrapViewports,
  checkWrapJob,
} from "./wrap-design.mjs";
import { renderSample, comparePng } from "./responsive-render.mjs";
import { openViewer, compareViewer, assertReadOnly } from "./browser.mjs";
import { waitFor } from "./protocol.mjs";

async function collect(mcp, base, sample, artifacts) {
  const context = await wrapContext(mcp, sample.projectId);
  const views = [];
  for (const [i, viewport] of wrapViewports.entries()) {
    const result = await renderSample(mcp, base, sample, viewport, artifacts);
    const job = await mcp.call("render_frame", {
      projectId: sample.projectId,
      jobId: result.jobId,
      includeScene: true,
    });
    checkWrapJob(job, i);
    for (const node of job.scene.nodes) {
      const c = context.nodes.find((n) => n.id === node.id);
      const { x, y, width, height } = c.computedBounds[i];
      assert.deepEqual({ x, y, width, height }, node.bounds);
    }
    views.push({ ...result, job });
  }
  return { context, views };
}
export async function collectWrap(mcp, base, artifacts) {
  const project = await mcp.call("create_project", {
    slug: "e2e-wrap",
    name: "Temporary wrap acceptance",
  });
  const sample = {
    kind: "wrap",
    projectId: project.projectId,
    frameId: "frame",
    nodeCount: 8,
  };
  const args = {
    projectId: sample.projectId,
    expectedRevision: 0,
    idempotencyKey: "wrap-fixture",
    operations: wrapOperations(),
  };
  const applied = await mcp.call("apply_batch", args);
  assert.equal(applied.committedRevision, 1);
  assert.equal((await mcp.call("apply_batch", args)).committedRevision, 1);
  const subtree = await mcp.call("get_subtree", {
    projectId: sample.projectId,
    nodeId: "frame",
  });
  assert.equal(subtree.revision, 1);
  assert.equal(
    subtree.nodes.find((n) => n.id === "cards").props.layout.wrap,
    true,
  );
  return {
    ...sample,
    subtree,
    ...(await collect(mcp, base, sample, artifacts)),
  };
}
export async function verifyWrapRepeat(mcp, base, browser, before, artifacts) {
  const subtree = await mcp.call("get_subtree", {
    projectId: before.projectId,
    nodeId: "frame",
  });
  assert.deepEqual(subtree, before.subtree);
  const after = await collect(mcp, base, before);
  assert.deepEqual(after.context.nodes, before.context.nodes);
  const view = await openViewer(browser, base);
  const report = {
    projectId: before.projectId,
    revision: 1,
    reflow: true,
    viewports: [],
  };
  try {
    await view.page.getByLabel("Project", { exact: true }).click();
    await view.page
      .locator(`[role="option"][data-value="${before.projectId}"]`)
      .click();
    for (const [i, current] of after.views.entries()) {
      const old = before.views[i];
      assert.notEqual(current.jobId, old.jobId);
      assert.equal(current.sceneKey, old.sceneKey);
      assert.deepEqual(current.bounds, old.bounds);
      const pixels = await comparePng(
        view.page,
        old.pngData,
        current.pngData,
        current.viewport,
      );
      let viewer;
      if (i !== 1) {
        await view.page.getByLabel("Viewport", { exact: true }).click();
        await view.page
          .getByRole("option", { name: i === 0 ? "Frame size" : /^Mobile/ })
          .click();
        await waitFor(
          async () =>
            view.scenes.some(
              (s) => s.provenance.sourceSceneKey === current.sceneKey,
            ),
          "wrap viewer scene",
        );
        viewer = await compareViewer(view, current.job);
        await view.page.screenshot({
          path: fileURLToPath(
            new URL(`wrap-viewer-${current.viewport.width}.png`, artifacts),
          ),
        });
      }
      report.viewports.push({
        viewport: current.viewport,
        rowHeight: [128, 224, 320][i],
        bounds: current.bounds,
        pixels,
        viewer,
        sceneKey: current.sceneKey,
        restartedJob: current.jobId,
      });
    }
    assertReadOnly(view);
    report.viewerRequests = view.requests;
    report.browserErrors = view.errors;
  } finally {
    await view.page.close();
  }
  await writeFile(
    new URL("wrap.json", artifacts),
    JSON.stringify(report, null, 2),
  );
  return report;
}
