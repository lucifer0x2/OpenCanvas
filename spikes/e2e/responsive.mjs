import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import {
  welcomeOperations,
  FRAME as FREE_FRAME,
} from "../../samples/welcome/design.mjs";
import {
  stacksOperations,
  FRAME as STACKS_FRAME,
} from "../../samples/welcome-stacks/design.mjs";
import { EPSILON, compareBounds, horizontalOverflow } from "./bounds.mjs";
import { renderSample, comparePng } from "./responsive-render.mjs";

const viewports = [
  { width: 1040, height: 720 },
  { width: 800, height: 720 },
  { width: 390, height: 844 },
];
const limitations = {
  free: "All descendants retain fixed authored bounds, including sidebar height 720; clipping hides overflow. No responsive/mobile acceptance.",
  stacks:
    "Body minWidth 480 plus sidebar 176 requires 656px. At 800, fixed plot overflows its card but stays inside FRAME. At 390, minima cause FRAME overflow; text wrapping can overlap vertically (outside this horizontal check). No mobile acceptance.",
};

function chartOverflow(bounds, operations) {
  const ids = new Set(["weekly-chart"]);
  // Authored operations are parent-first, including all structural wrappers.
  for (const n of operations.filter((op) => op.op === "create_node")) {
    if (ids.has(n.parentId)) ids.add(n.id);
  }
  const card = bounds.find((n) => n.id === "weekly-chart").bounds;
  const local = bounds
    .filter((n) => n.id !== "weekly-chart" && ids.has(n.id))
    .map((n) => ({
      id: n.id,
      bounds: { ...n.bounds, x: n.bounds.x - card.x },
    }));
  return {
    containerId: "weekly-chart",
    width: card.width,
    overflow: horizontalOverflow(local, { width: card.width }),
  };
}

export async function collectResponsive(mcp, base, artifacts) {
  const known = JSON.parse(
    await readFile(new URL("known-overflow.json", import.meta.url)),
  );
  const samples = [];
  for (const [kind, frameId, make] of [
    ["free", FREE_FRAME, welcomeOperations],
    ["stacks", STACKS_FRAME, stacksOperations],
  ]) {
    const operations = make();
    const nodes = operations.filter(
      (op) => op.op === "create_node" && op.kind !== "page",
    );
    const project = await mcp.call("create_project", {
      slug: `e2e-${kind}`,
      name: `Temporary ${kind} acceptance`,
    });
    const sample = {
      kind,
      projectId: project.projectId,
      frameId,
      nodeCount: nodes.length,
      viewports: [],
    };
    await mcp.call("apply_batch", {
      projectId: sample.projectId,
      expectedRevision: 0,
      idempotencyKey: `responsive-${kind}`,
      operations,
    });
    for (const viewport of viewports) {
      const result = await renderSample(mcp, base, sample, viewport, artifacts);
      assert.deepEqual(
        result.bounds.map((n) => n.id).sort(),
        nodes
          .filter((n) => n.id !== frameId)
          .map((n) => n.id)
          .sort(),
      );
      const overflow = horizontalOverflow(result.bounds, viewport);
      // Explicit node IDs AND px from prior Task02 evidence; any added/removed/drifting overflow fails.
      assert.deepEqual(
        overflow,
        known[kind][viewport.width],
        `${kind} ${viewport.width}: unreviewed FRAME overflow change`,
      );
      sample.viewports.push({
        ...result,
        descendantCount: result.bounds.length,
        frameOverflow: overflow,
        horizontalFit: overflow.length === 0,
        knownOverflowMatches: true,
        cardOverflow: chartOverflow(result.bounds, operations),
      });
    }
    const [desktop, medium, narrow] = sample.viewports;
    sample.reflow = sample.viewports.slice(1).map((v) => ({
      from: desktop.viewport,
      to: v.viewport,
      ...compareBounds(desktop.bounds, v.bounds),
    }));
    sample.reflowBetween800And390 = compareBounds(medium.bounds, narrow.bounds);
    assert.ok(sample.reflow.every((r) => r.reflow === (kind === "stacks")));
    const bounds = (v, id) => v.bounds.find((n) => n.id === id).bounds;
    assert.deepEqual(
      sample.viewports.map((v) => bounds(v, "sidebar").width),
      [176, 176, 176],
    );
    if (kind === "free") {
      assert.deepEqual(bounds(narrow, "sidebar"), {
        x: 0,
        y: 0,
        width: 176,
        height: 720,
      });
      assert.deepEqual(bounds(narrow, "focus"), {
        x: 744,
        y: 148,
        width: 248,
        height: 112,
      });
      for (const [id, rightPx] of [
        ["date", 238],
        ["greeting", 418],
        ["new-task-button", 602],
        ["focus", 602],
      ]) {
        assert.equal(
          narrow.frameOverflow.find((n) => n.nodeId === id)?.rightPx,
          rightPx,
        );
      }
    } else {
      assert.deepEqual(
        sample.viewports.map((v) => bounds(v, "body").width),
        [864, 624, 480],
      );
      assert.deepEqual(
        sample.viewports.map((v) => bounds(v, "completed").width),
        [248, 168, 120],
      );
      assert.equal(
        narrow.frameOverflow.find((n) => n.nodeId === "body")?.rightPx,
        266,
      );
      assert.equal(medium.frameOverflow.length, 0);
      assert.ok(
        medium.cardOverflow.overflow.some(
          (n) => n.nodeId === "chart-grid-0" && n.rightPx > 0,
        ),
      );
      // These width changes prove real co-sizing at equal viewport HEIGHT, beyond sidebar height changes.
      assert.ok(
        sample.reflow[0].changes
          .find((n) => n.nodeId === "body")
          ?.changedDimensions.includes("width"),
      );
    }
    sample.designStatus =
      kind === "free" ? "fixed; no reflow" : "reflow with known fit limits";
    sample.mobileAccepted = false;
    sample.limitations = limitations[kind];
    samples.push(sample);
  }
  return {
    epsilonLogicalPx: EPSILON,
    scope:
      "ALL frame descendants; root excluded; x/y/width/height; before clipping",
    knownOverflowPolicy:
      "Exact node IDs and px in known-overflow.json, frozen from Task02 evidence; changes fail for review, never auto-regenerated by test.",
    samples,
  };
}

export async function verifyResponsiveRepeat(
  mcp,
  base,
  browser,
  report,
  artifacts,
) {
  const page = await browser.newPage();
  try {
    for (const sample of report.samples) {
      for (const viewport of sample.viewports) {
        const repeat = await renderSample(mcp, base, sample, viewport.viewport);
        assert.notEqual(
          repeat.jobId,
          viewport.jobId,
          "restart must create a fresh render job",
        );
        assert.equal(repeat.sceneKey, viewport.sceneKey);
        assert.deepEqual(repeat.bounds, viewport.bounds);
        viewport.pixels = await comparePng(
          page,
          viewport.pngData,
          repeat.pngData,
          viewport.viewport,
        );
        viewport.pixelPass = true;
        viewport.repeat = {
          jobId: repeat.jobId,
          sceneKey: repeat.sceneKey,
          png: repeat.png,
          provenance: repeat.provenance,
          independentAfterRestart: true,
        };
        delete viewport.pngData;
      }
      sample.pixelPass = sample.viewports.every((v) => v.pixelPass);
    }
  } finally {
    await page.close();
  }
  report.comparison = viewports.map((viewport, i) => ({
    viewport,
    ...Object.fromEntries(
      report.samples.map((s) => [
        s.kind,
        {
          pixelPass: s.pixelPass,
          descendantCount: s.viewports[i].descendantCount,
          frameOverflowCount: s.viewports[i].frameOverflow.length,
          horizontalFit: s.viewports[i].horizontalFit,
          reflowFrom1040: i ? s.reflow[i - 1].reflow : null,
          mobileAccepted: s.mobileAccepted,
        },
      ]),
    ),
  }));
  await writeFile(
    new URL("responsive.json", artifacts),
    JSON.stringify(report, null, 2),
  );
  return report;
}
