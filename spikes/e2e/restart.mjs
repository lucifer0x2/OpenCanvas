import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { connectMcp, json } from "./protocol.mjs";
import { startServer } from "./server.mjs";
import { render } from "./design.mjs";
import { ready, compareViewer } from "./browser.mjs";

export async function restartAndVerify({
  server,
  mcp,
  dataRoot,
  view,
  projectId,
  assetId,
  job,
  artifacts,
}) {
  const base = server.base;
  const route = `/api/v1/projects/${projectId}/snapshot`;
  const assetURL = `${base}/api/v1/projects/${projectId}/assets/${assetId}`;
  const before = await json(base, route);
  const assetBefore = Buffer.from(await (await fetch(assetURL)).arrayBuffer());
  await mcp.close();
  await view.page.goto("about:blank");
  const port = server.port;
  await server.stop();
  const restarted = await startServer(dataRoot, port);
  let reconnected;
  try {
    assert.deepEqual(await json(base, route), before);
    assert.deepEqual(
      Buffer.from(await (await fetch(assetURL)).arrayBuffer()),
      assetBefore,
    );
    reconnected = await connectMcp(base);
    const restored = await render(reconnected, base, projectId, job.revision);
    assert.equal(restored.sceneKey, job.sceneKey);
    // Restore desktop width so the real inspector is visible after app mount.
    await view.page.setViewportSize({ width: 1440, height: 1000 });
    await view.page.goto(base);
    await ready(view.page, job.revision);
    await view.page
      .getByRole("heading", { name: "Headline", exact: true })
      .waitFor();
    await view.page.reload();
    await ready(view.page, job.revision);
    await view.page
      .getByRole("heading", { name: "Headline", exact: true })
      .waitFor();
    const pixels = await compareViewer(view, restored);
    await view.page.screenshot({
      path: fileURLToPath(new URL("restarted.png", artifacts)),
    });
    return { server: restarted, mcp: reconnected, pixels };
  } catch (error) {
    await reconnected?.close();
    await restarted.stop();
    throw error;
  }
}
