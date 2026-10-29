import assert from "node:assert/strict";
import { mkdtemp, mkdir, rm, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { createHash } from "node:crypto";
import { connectMcp, waitFor } from "./protocol.mjs";
import { startServer, workspace } from "./server.mjs";
import { createDesign, render, token } from "./design.mjs";
import { restartAndVerify } from "./restart.mjs";
import { readback, inspect } from "./readback.mjs";
import { collectResponsive, verifyResponsiveRepeat } from "./responsive.mjs";
import { collectWrap, verifyWrapRepeat } from "./wrap.mjs";
import { verifyLiveWorkspace } from "./live-workspace.mjs";
import {
  openViewer,
  ready,
  compareViewer,
  interact,
  assertReadOnly,
} from "./browser.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= join(workspace, ".runtime/browsers");
const { chromium } = await import("playwright");
const artifacts = new URL("../../artifacts/e2e/", import.meta.url);
await mkdir(artifacts, { recursive: true });
const dataRoot = await mkdtemp(join(tmpdir(), "opencanvas-live-e2e-"));
let server, mcp, browser;
const evidence = {
  startedAt: new Date().toISOString(),
  checks: [],
  screenshots: [],
};
try {
  server = await startServer(dataRoot);
  const base = server.base;
  mcp = await connectMcp(base);
  browser = await chromium.launch({ headless: true });
  const generator = await browser.newPage();
  const image = await generator.evaluate(() => {
    const canvas = document.createElement("canvas");
    canvas.width = 96;
    canvas.height = 64;
    const c = canvas.getContext("2d");
    c.fillStyle = "#e6a43a";
    c.fillRect(0, 0, 48, 64);
    c.fillStyle = "#2266cc";
    c.fillRect(48, 0, 48, 64);
    return canvas.toDataURL("image/png").split(",")[1];
  });
  await generator.close();
  const design = await createDesign(mcp, image);
  const { projectId, assetId } = design;
  await readback({ mcp, base, dataRoot, projectId, assetId, artifacts });
  evidence.checks.push(
    "actual MCP initialize/session/initialized/list/call",
    "SQLite structure + semantic readback",
    "discarded response identical retry r2",
    "changed-body retry rejected",
    "invalid batch rollback r2",
  );
  const job2 = await render(mcp, base, projectId, 2);
  await inspect(mcp, job2);
  assert.equal(
    job2.scene.nodes.find((node) => node.id === "rectangle").fill,
    "#2266cc",
  );
  assert.ok(
    job2.scene.nodes.find((node) => node.id === "headline").lines.length > 1,
  );
  const view = await openViewer(browser, base);
  const pixels2 = await compareViewer(view, job2);
  await interact(view, artifacts);
  await mcp.call("apply_batch", {
    projectId,
    expectedRevision: 2,
    idempotencyKey: "brand-update",
    operations: [token("#c94b62")],
  });
  // No page reload: a production SSE commit must cause snapshot + scene refresh.
  await ready(view.page, 3);
  const job3 = await render(mcp, base, projectId, 3);
  assert.notEqual(job3.sceneKey, job2.sceneKey);
  assert.equal(
    job3.scene.nodes.find((node) => node.id === "rectangle").fill,
    "#c94b62",
  );
  const pixels3 = await compareViewer(view, job3);
  await view.page.screenshot({
    path: fileURLToPath(new URL("token-update.png", artifacts)),
  });
  await view.page.getByLabel("Viewport", { exact: true }).click();
  await view.page.getByRole("option", { name: /^Mobile/ }).click();
  const mobile = await render(mcp, base, projectId, 3, 390, 844);
  await waitFor(
    async () =>
      view.scenes.some((s) => s.provenance.sourceSceneKey === mobile.sceneKey),
    "mobile scene",
  );
  const mobilePixels = await compareViewer(view, mobile);
  assert.ok(
    mobile.scene.nodes.find((node) => node.id === "headline").lines.length >
      job3.scene.nodes.find((node) => node.id === "headline").lines.length,
  );
  await view.page.setViewportSize({ width: 390, height: 844 });
  await view.page.getByRole("button", { name: "Hide layers panel" }).click();
  await view.page.getByRole("button", { name: "Hide inspector panel" }).click();
  assert.equal(
    await view.page.evaluate(
      () => document.documentElement.scrollWidth > innerWidth,
    ),
    false,
  );
  await view.page.screenshot({
    path: fileURLToPath(new URL("mobile.png", artifacts)),
  });
  evidence.checks.push(
    "MCP -> strict PNG -> live viewer exact scene and RGBA",
    "production SSE token update without refresh",
    "390x844 viewport wraps text + exact PNG pixels",
    "selection/zoom/pan/fit + narrow shell",
  );
  const pngResponse = await fetch(job3.imageUrl);
  assert.equal(pngResponse.headers.get("content-type"), "image/png");
  const png = Buffer.from(await pngResponse.arrayBuffer());
  await writeFile(new URL("frame.png", artifacts), png);
  const responsive = await collectResponsive(mcp, base, artifacts);
  const wrap = await collectWrap(mcp, base, artifacts);
  const restarted = await restartAndVerify({
    server,
    mcp,
    dataRoot,
    view,
    projectId,
    assetId,
    job: job3,
    artifacts,
  });
  server = restarted.server;
  mcp = restarted.mcp;
  const restartPixels = restarted.pixels;
  await verifyResponsiveRepeat(
    mcp,
    server.base,
    browser,
    responsive,
    artifacts,
  );
  evidence.wrap = await verifyWrapRepeat(
    mcp,
    server.base,
    browser,
    wrap,
    artifacts,
  );
  evidence.liveWorkspace = await verifyLiveWorkspace(browser);
  assertReadOnly(view);
  evidence.checks.push(
    "process termination/restart + browser reload retains IDs/content/refs/revision/selection",
    "stable scene key after restart",
    "viewer GET-only API requests and no browser errors",
    "free + stacks actual MCP: all descendant FRAME overflow at 1040/800/390",
    "full-scope reflow excludes root: free false, stacks true with width changes",
    "free + stacks independent post-restart PNG pixels at all three viewports",
    "row wrap actual MCP/context/inspect/PNG at 1040/640/390, persisted restart and GET-only shared-render viewer",
  );
  Object.assign(evidence, {
    completedAt: new Date().toISOString(),
    browser: browser.version(),
    protocol: mcp.protocol,
    server: mcp.server,
    tools: mcp.tools.map((tool) => tool.name),
    projectId,
    revisions: [2, 3],
    sceneKeys: {
      r2: job2.sceneKey,
      r3: job3.sceneKey,
      mobile: mobile.sceneKey,
    },
    provenance: job3.provenance,
    responsive: {
      report: "responsive.json",
      comparison: responsive.comparison,
    },
    pixels: {
      r2: pixels2,
      r3: pixels3,
      mobile: mobilePixels,
      restart: restartPixels,
    },
    png: {
      sha256: createHash("sha256").update(png).digest("hex"),
      bytes: png.length,
      width: png.readUInt32BE(16),
      height: png.readUInt32BE(20),
    },
    apiRequests: view.requests,
    screenshots: [
      "desktop.png",
      "token-update.png",
      "mobile.png",
      "restarted.png",
    ],
  });
  await writeFile(
    new URL("scene.json", artifacts),
    JSON.stringify(job3, null, 2),
  );
} catch (error) {
  await writeFile(
    new URL("failure.json", artifacts),
    JSON.stringify({ error: error.stack, server: server?.output() }, null, 2),
  );
  throw error;
} finally {
  const cleanup = await Promise.allSettled([
    mcp?.close(),
    browser?.close(),
    server?.stop(),
  ]);
  await rm(dataRoot, { recursive: true, force: true });
  assert.ok(
    cleanup.every((result) => result.status === "fulfilled"),
    "E2E process cleanup failed",
  );
}
evidence.temporaryDataRemoved = true;
await writeFile(
  new URL("evidence.json", artifacts),
  JSON.stringify(evidence, null, 2),
);
await rm(new URL("failure.json", artifacts), { force: true });
console.log(JSON.stringify(evidence, null, 2));
