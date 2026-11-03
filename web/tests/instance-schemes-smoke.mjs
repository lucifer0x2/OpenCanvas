// Real preview checks for scoped motion and state declarations introduced only by an override.
// Requires synchronized debug server/layout-step, renderer/dist and web/dist.
import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { connectMcp } from "../../spikes/e2e/protocol.mjs";
import { startServer } from "../../spikes/e2e/server.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const { chromium } = await import("playwright");
const dataRoot = await mkdtemp(join(tmpdir(), "opencanvas-instance-schemes-"));
let server, browser, mcp;
const node = (id, kind, parentId, props) => ({
  op: "create_node",
  id,
  kind,
  parentId,
  props,
});
try {
  server = await startServer(dataRoot);
  mcp = await connectMcp(server.base);
  const { projectId } = await mcp.call("create_project", {
    slug: "instance-schemes",
    name: "Instance schemes",
  });
  await mcp.call("apply_batch", {
    projectId,
    expectedRevision: 0,
    idempotencyKey: "fixture",
    operations: [
      {
        op: "upsert_token",
        token: {
          id: "ink",
          name: "Ink",
          type: "color",
          value: {
            default: "#111111",
            themes: { night: "#eeeeee", day: "#008800" },
          },
        },
      },
      {
        op: "upsert_token",
        token: {
          id: "duration",
          name: "Duration",
          type: "number",
          value: { default: 300, themes: { night: 750 } },
        },
      },
      node("page", "page", null, {}),
      node("frame", "frame", "page", {
        width: 200,
        height: 100,
        theme: "night",
        fill: "#fff",
        animations: [
          {
            name: "scheme",
            duration: { tokenId: "duration" },
            keyframes: [{ fill: { tokenId: "ink" } }],
          },
        ],
      }),
      node("master", "frame", "page", {
        component: { name: "Tile" },
        width: 80,
        height: 80,
        fill: { tokenId: "ink" },
      }),
      node("box", "rectangle", "master", {
        x: 10,
        y: 10,
        width: 30,
        height: 30,
        fill: { tokenId: "ink" },
      }),
      node("a", "instance", "frame", {
        componentId: "master",
        overrides: { box: { states: { hover: { fill: "#ff0000" } } } },
      }),
      node("b", "instance", "frame", {
        componentId: "master",
        x: 100,
        theme: "day",
      }),
    ],
  });
  await mcp.close();
  mcp = undefined;
  browser = await chromium.launch();
  const page = await browser.newPage({ viewport: { width: 900, height: 600 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.goto(`${server.base}/preview?project=${projectId}&frame=frame`);
  await page.waitForSelector('[data-oc-id="a/box"]>.oc-box', {
    timeout: 60000,
  });
  const fill = (id) =>
    page.$eval(
      `[data-oc-id="${id}"]>.oc-box`,
      (el) => getComputedStyle(el).backgroundColor,
    );
  assert.equal(await fill("a/box"), "rgb(238, 238, 238)");
  assert.equal(await fill("b"), "rgb(0, 136, 0)");
  await page.locator('[data-oc-id="a/box"]').hover();
  assert.equal(await fill("a/box"), "rgb(255, 0, 0)");
  const motion = await page.evaluate(() => {
    const animation = document
      .getAnimations()
      .find((a) => a.id === "frame:scheme:fill");
    assertAnimation(animation);
    function assertAnimation(value) {
      if (!value) throw new Error("Scoped animation missing");
    }
    return {
      duration: animation.effect.getComputedTiming().duration,
      colors: animation.effect.getKeyframes().map((k) => k.backgroundColor),
    };
  });
  assert.equal(motion.duration, 750);
  assert.equal(motion.colors.at(-1), "rgb(238, 238, 238)");
  assert.deepEqual(errors, []);
  console.log(
    "ok - scoped preview paint, override-only hover state and scoped animation timing/color",
  );
} finally {
  await mcp?.close();
  await browser?.close();
  await server?.stop();
  await rm(dataRoot, { recursive: true, force: true });
}
