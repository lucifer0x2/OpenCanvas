// Actual viewer, backend, MCP, Chromium and temporary data. Never touches the live server.
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { connectMcp, json } from "../../spikes/e2e/protocol.mjs";
import { startServer } from "../../spikes/e2e/server.mjs";
import { createActionFixture } from "./node-actions-fixture.mjs";
import { verifyDevExports } from "./node-actions-dev.mjs";
import { verifyCopy } from "./node-actions-copy.mjs";
import { verifyUnavailableCanvas } from "./node-actions-unavailable.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const { chromium } = await import("playwright");
const root = await mkdtemp(join(tmpdir(), "opencanvas-node-actions-"));
const server = await startServer(root);
let browser, mcp;
const step = (name) => console.log(`ok - ${name}`);
try {
  mcp = await connectMcp(server.base);
  const projectId = await createActionFixture(mcp);
  browser = await chromium.launch();
  const context = await browser.newContext({
    viewport: { width: 1300, height: 900 },
    permissions: ["clipboard-read", "clipboard-write"],
  });
  const page = await context.newPage();
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  await page.addInitScript(() =>
    localStorage.setItem(
      "opencanvas.selection",
      JSON.stringify({
        projectId: "wrong",
        pageId: "wrong",
        frameId: "wrong",
        nodeId: "wrong",
      }),
    ),
  );
  const address = (id) =>
    `${server.base}/?project=${projectId}&page=page&frame=frame&node=${encodeURIComponent(id)}`;
  await page.goto(address("instance/label"));
  await page
    .getByRole("heading", { name: "Placed label", exact: true })
    .waitFor();
  await page.getByText("Scene r1", { exact: true }).waitFor({ timeout: 60000 });
  assert.equal(
    await page
      .getByRole("combobox", { name: "Page", exact: true })
      .getAttribute("value"),
    "page",
  );
  assert.equal(
    await page
      .getByRole("combobox", { name: "Frame", exact: true })
      .getAttribute("value"),
    "frame",
  );
  assert.equal(
    await page.evaluate(() =>
      window.Konva.stages[0].find("Rect").some((r) => r.stroke() === "#1672e8"),
    ),
    true,
  );
  await page.getByRole("button", { name: "Copy link", exact: true }).click();
  const copied = await page.evaluate(() => navigator.clipboard.readText());
  assert.equal(copied, address("instance/label"));
  const identity = await mcp.call("find_nodes", {
    projectId,
    ids: [new URL(copied).searchParams.get("node")],
  });
  assert.ok(JSON.stringify(identity).includes("instance/label"));
  const fresh = await browser.newPage({
    viewport: { width: 1300, height: 900 },
  });
  await fresh.goto(copied);
  await fresh
    .getByRole("heading", { name: "Placed label", exact: true })
    .waitFor();
  await fresh.close();
  step(
    "fresh deep link overrides stored scope, highlights virtual child and resolves through MCP",
  );
  const [virtualFile] = await Promise.all([
    page.waitForEvent("download"),
    page.getByRole("button", { name: "Export SVG", exact: true }).click(),
  ]);
  assert.ok(
    (await readFile(await virtualFile.path(), "utf8")).includes("Placed"),
  );
  step("virtual child SVG export uses the placed override text");

  await page.getByRole("button", { name: "Export box", exact: true }).click();
  const coords = await page.evaluate(() => {
    const st = window.Konva.stages[0],
      box = st.container().getBoundingClientRect();
    return {
      blank: { x: box.x + 10, y: box.y + 10 },
      node: {
        x: box.x + st.x() + 60 * st.scaleX(),
        y: box.y + st.y() + 50 * st.scaleY(),
      },
    };
  });
  await page.mouse.click(coords.node.x, coords.node.y, { button: "right" });
  await page.getByRole("menu", { name: "Element actions" }).waitFor();
  if (process.env.OPENCANVAS_ACTION_SCREENSHOT)
    await page.screenshot({ path: process.env.OPENCANVAS_ACTION_SCREENSHOT });
  assert.equal(
    await page
      .getByRole("heading", { name: "Export box", exact: true })
      .count(),
    1,
  );
  await page.keyboard.press("ArrowDown");
  assert.equal(
    await page.evaluate(() => document.activeElement.textContent),
    "Export PNG",
  );
  await page.keyboard.press("Escape");
  assert.equal(await page.getByRole("menu").count(), 0);
  await page
    .getByRole("button", { name: "Placed card", exact: true })
    .click({ button: "right" });
  assert.equal(
    await page
      .getByRole("heading", { name: "Placed card", exact: true })
      .count(),
    1,
  );
  await page.locator(".statusbar").click();
  assert.equal(await page.getByRole("menu").count(), 0);
  step(
    "canvas and Layers context menu select their targets, support arrows/Escape/outside click",
  );

  await verifyCopy(page, address);
  await page.mouse.click(coords.blank.x, coords.blank.y);
  assert.equal(await page.locator(".inspector h2").count(), 0);
  assert.equal(await page.evaluate(() => document.activeElement.id), "");
  await mcp.call("apply_batch", {
    projectId,
    expectedRevision: 1,
    idempotencyKey: "sse",
    operations: [
      { op: "update_node", id: "box", props: { name: "Export box updated" } },
    ],
  });
  await page.getByText("Authored r2", { exact: true }).waitFor();
  assert.equal(await page.locator(".inspector h2").count(), 0);
  step(
    "Cmd/Ctrl+C copies IDs, input copy stays native, blank click blurs/deselects across SSE",
  );

  await page
    .getByRole("button", { name: "Export box updated", exact: true })
    .click();
  await page.mouse.move(coords.blank.x, coords.blank.y);
  await page.mouse.down();
  await page.mouse.move(coords.blank.x + 50, coords.blank.y + 30, { steps: 5 });
  await page.mouse.up();
  assert.equal(
    await page.locator(".inspector h2").textContent(),
    "Export box updated",
  );
  const before = (
    await json(server.base, `/api/v1/projects/${projectId}/snapshot`)
  ).revision;
  for (const format of ["PNG", "SVG"]) {
    const responsePromise = page.waitForResponse((r) =>
      /\/files\/0$/.test(r.url()),
    );
    const [download] = await Promise.all([
      page.waitForEvent("download"),
      page
        .getByRole("button", { name: `Export ${format}`, exact: true })
        .click(),
    ]);
    const response = await responsePromise;
    assert.equal(
      response.headers()["content-type"],
      format === "PNG" ? "image/png" : "image/svg+xml",
    );
    const bytes = await readFile(await download.path());
    if (format === "PNG") assert.equal(bytes.subarray(1, 4).toString(), "PNG");
    else assert.ok(bytes.toString().includes("<svg"));
    assert.ok(bytes.length > 100);
  }
  assert.equal(
    (await json(server.base, `/api/v1/projects/${projectId}/snapshot`))
      .revision,
    before,
  );
  step(
    "pan preserves selection; Inspector downloads actual PNG/SVG bytes with correct MIME and unchanged revision",
  );
  await page.getByRole("button", { name: "Empty box", exact: true }).click();
  await page.getByRole("button", { name: "Export PNG", exact: true }).click();
  await page
    .getByText("Node has zero width or height in its frame's layout", {
      exact: true,
    })
    .waitFor();
  assert.equal(
    await page
      .getByRole("button", { name: "Export PNG", exact: true })
      .isEnabled(),
    true,
  );
  step("failed export shows the actual backend error and re-enables actions");
  await verifyUnavailableCanvas(page, address, projectId);
  await page.goto(address("deleted"));
  await page
    .getByText("The selected layer was removed. Showing its frame.", {
      exact: true,
    })
    .waitFor();
  await page
    .getByRole("heading", { name: "Target frame", exact: true })
    .waitFor();
  assert.deepEqual(errors, []);
  step("stale link reports missing node and viewer has no page errors");
  await verifyDevExports(browser, server.base, projectId);
} finally {
  await browser?.close();
  await mcp?.close();
  await server.stop();
  await rm(root, { recursive: true, force: true });
  console.log(`cleaned temporary backend port ${server.port} and data root`);
}
