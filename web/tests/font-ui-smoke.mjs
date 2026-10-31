// Real imports/downloads on a temporary production backend + Vite source viewer.
import assert from "node:assert/strict";
import { mkdtemp, readFile, rm, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { connectMcp, json } from "../../spikes/e2e/protocol.mjs";
import { startServer } from "../../spikes/e2e/server.mjs";
import { startFontUi } from "./font-ui-server.mjs";
import { verifyFontUi } from "./font-ui-browser.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const { chromium } = await import("playwright");
const root = await mkdtemp(join(tmpdir(), "opencanvas-font-ui-"));
const artifacts =
  process.env.OPENCANVAS_FONT_ARTIFACTS ??
  join(tmpdir(), "opencanvas-font-ui-evidence");
await mkdir(artifacts, { recursive: true });
let server, ui, browser, mcp;
try {
  server = await startServer(root);
  ui = await startFontUi(server.base);
  mcp = await connectMcp(server.base);
  const { projectId } = await mcp.call("create_project", {
    slug: "font-ui",
    name: "Font portability",
  });
  const other = await mcp.call("create_project", {
    slug: "font-other",
    name: "Other project",
  });
  const fixture = await readFile(
    new URL("../../renderer/fonts/NotoSans.ttf", import.meta.url),
  );
  browser = await chromium.launch();
  const page = await browser.newPage({
    viewport: { width: 1300, height: 1000 },
  });
  const errors = [];
  const consoleErrors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("console", (message) => {
    if (
      message.type() === "error" &&
      !/net::ERR_FAILED|status of 409/.test(message.text())
    )
      consoleErrors.push(message.text());
  });
  const controls = await verifyFontUi(
    page,
    ui.base,
    projectId,
    fixture,
    artifacts,
  );
  const endpoint = `**/api/v1/projects/${projectId}/fonts`;
  const requests = [];
  let interrupt = true;
  await page.route(endpoint, async (route) => {
    if (route.request().method() !== "POST") return route.continue();
    requests.push(route.request().postDataJSON());
    if (interrupt) {
      interrupt = false;
      const response = await route.fetch();
      assert.equal(response.status(), 200, await response.text());
      return route.abort("failed"); // Server committed: emulate a lost response, then retry exact body.
    }
    return route.continue();
  });
  await controls.dialog
    .getByRole("button", { name: "Add font", exact: true })
    .click();
  await controls.dialog
    .getByRole("button", { name: "Retry same import" })
    .click();
  await controls.dialog
    .getByText(
      "Portable Noto was added to this project. No design fonts were replaced.",
    )
    .waitFor();
  assert.deepEqual(
    requests[0],
    requests[1],
    "ambiguous retry reuses immutable body/key/ID",
  );
  const result = await json(server.base, `/api/v1/projects/${projectId}/fonts`);
  assert.equal(result.fonts.length, 2);
  assert.equal(result.revision, 1);
  const portable = result.fonts.find((font) => font.family === "Portable Noto");
  assert.equal(portable.source, "project");
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    controls.dialog
      .getByRole("button", { name: "Download Portable Noto, 400 normal" })
      .click(),
  ]);
  assert.equal(
    controls.hash(await readFile(await download.path())),
    controls.hash(fixture),
  );
  console.log(
    "ok - bundled and imported original downloads match SHA256; committed lost-response retry imports once",
  );
  await page.unroute(endpoint);
  // Change the actual project only after the stale POST body has already left the UI.
  await controls.file.setInputFiles({
    name: "Conflict.ttf",
    mimeType: "font/ttf",
    buffer: fixture,
  });
  await controls.dialog
    .getByLabel("Font family", { exact: true })
    .fill("Conflict Noto");
  let conflict = true;
  let rejectedId;
  await page.route(endpoint, async (route) => {
    if (route.request().method() === "POST" && conflict) {
      conflict = false;
      rejectedId = route.request().postDataJSON().font.id;
      await mcp.call("apply_batch", {
        projectId,
        expectedRevision: 1,
        idempotencyKey: "font-conflict",
        operations: [
          {
            op: "create_node",
            id: "page",
            kind: "page",
            parentId: null,
            props: { name: "Concurrent page" },
          },
        ],
      });
    }
    return route.continue();
  });
  await controls.dialog
    .getByRole("button", { name: "Add font", exact: true })
    .click();
  await controls.dialog
    .getByText(/This project changed before the import/)
    .waitFor();
  const added = page.waitForResponse(
    (response) =>
      response.request().method() === "POST" &&
      response.url().endsWith("/fonts"),
  );
  await controls.dialog
    .getByRole("button", { name: "Add font", exact: true })
    .click();
  assert.equal((await added).status(), 200);
  await controls.dialog.getByText(/Conflict Noto was added/).waitFor();
  const final = await json(server.base, `/api/v1/projects/${projectId}/fonts`);
  assert.equal(final.fonts.length, 3);
  assert.equal(final.revision, 3);
  assert.notEqual(
    final.fonts.find((font) => font.family === "Conflict Noto").id,
    rejectedId,
  );
  console.log(
    "ok - real stale revision conflict refreshes, explicit resubmit uses a new ID and preserves previous font",
  );
  await page.unroute(endpoint);
  await page.reload();
  await page.getByRole("button", { name: "Fonts", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByText("Portable Noto", { exact: true })
    .waitFor();
  await page.setViewportSize({ width: 390, height: 700 });
  const dialog = page.getByRole("dialog");
  await dialog.evaluate((element) => {
    element.scrollTop = 0;
  });
  await page.screenshot({ path: `${artifacts}/font-narrow-library.png` });
  await dialog.getByLabel("Font family", { exact: true }).fill("Narrow family");
  await dialog.getByLabel("License (optional)").focus();
  await page.screenshot({ path: `${artifacts}/font-narrow.png` });
  assert.equal(
    await dialog.evaluate((el) => el.scrollWidth <= el.clientWidth),
    true,
  );
  const rect = await dialog.boundingBox();
  assert.ok(
    rect.x >= 0 &&
      rect.x + rect.width <= 390 &&
      rect.y >= 0 &&
      rect.y + rect.height <= 700,
  );
  await dialog.getByRole("button", { name: "Close fonts" }).click();
  assert.equal(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
    true,
  );
  await page.getByRole("combobox", { name: "Project", exact: true }).click();
  await page
    .getByRole("option", { name: "Other project", exact: true })
    .click();
  await page.getByRole("button", { name: "Fonts", exact: true }).click();
  await page
    .getByRole("dialog")
    .getByText("OpenCanvas Noto Sans", { exact: true })
    .waitFor();
  assert.equal(
    await page
      .getByRole("dialog")
      .getByText("Portable Noto", { exact: true })
      .count(),
    0,
  );
  assert.equal(
    await page.getByLabel("Font family", { exact: true }).inputValue(),
    "",
  );
  assert.equal(
    (await json(server.base, `/api/v1/projects/${other.projectId}/fonts`))
      .revision,
    0,
  );
  await page.close();
  await mcp.close();
  mcp = null;
  await ui.stop();
  ui = null;
  await server.stop();
  server = await startServer(root);
  assert.equal(
    (await json(server.base, `/api/v1/projects/${projectId}/fonts`)).fonts
      .length,
    3,
  );
  assert.deepEqual(errors, []);
  assert.deepEqual(consoleErrors, []);
  console.log(
    `ok - reload, backend restart, focus, Escape, modal selects, 390px layout and project isolation; screenshots ${artifacts}`,
  );
} finally {
  await mcp?.close();
  await browser?.close();
  await ui?.stop();
  await server?.stop();
  await rm(root, { recursive: true, force: true });
}
