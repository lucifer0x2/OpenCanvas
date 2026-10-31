// Real HTTPS viewer/preview + local MCP. No Funnel, Docker, fixed ports or user data.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdtemp, readFile, rm, mkdir } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { connectMcp } from "../../spikes/e2e/protocol.mjs";
import { publicViewerServer } from "./public-viewer-server.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const { chromium } = await import("playwright");
const root = await mkdtemp(join(tmpdir(), "opencanvas-public-smoke-"));
const artifacts =
  process.env.OPENCANVAS_PUBLIC_ARTIFACTS ??
  join(tmpdir(), "opencanvas-public-evidence");
await mkdir(artifacts, { recursive: true });
let server, browser, mcp;
try {
  server = await publicViewerServer(root);
  mcp = await connectMcp(server.local);
  const { projectId } = await mcp.call("create_project", {
    slug: "public-smoke",
    name: "Public smoke",
  });
  const font = await readFile(
    new URL("../../renderer/fonts/NotoSans.ttf", import.meta.url),
  );
  await mcp.call("import_asset", {
    projectId,
    expectedRevision: 0,
    idempotencyKey: "font",
    mediaType: "font/ttf",
    base64: font.toString("base64"),
    font: {
      id: "font_public",
      family: "Public Noto",
      weight: 400,
      style: "normal",
    },
  });
  await mcp.call("import_asset", {
    projectId,
    expectedRevision: 1,
    idempotencyKey: "image",
    mediaType: "image/png",
    base64:
      "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+aX1sAAAAASUVORK5CYII=",
  });
  const snapshot = await (
    await fetch(`${server.local}/api/v1/projects/${projectId}/snapshot`)
  ).json();
  const assetId = snapshot.assets.find(
    (asset) => asset.mediaType === "image/png",
  ).id;
  const committed = await mcp.call("apply_batch", {
    projectId,
    expectedRevision: 2,
    idempotencyKey: "design",
    operations: [
      {
        op: "create_node",
        id: "page",
        kind: "page",
        parentId: null,
        props: { name: "Home" },
      },
      {
        op: "create_node",
        id: "frame",
        kind: "frame",
        parentId: "page",
        props: {
          name: "Public frame",
          width: 320,
          height: 200,
          fill: "#ffffff",
        },
      },
      {
        op: "create_node",
        id: "text",
        kind: "text",
        parentId: "frame",
        props: {
          name: "Public label",
          text: "Public Việt",
          width: 280,
          height: "hug",
          fontId: "font_public",
          fontSize: 20,
          lineHeight: 28,
        },
      },
      {
        op: "create_node",
        id: "image",
        kind: "image",
        parentId: "frame",
        props: {
          name: "Public image",
          assetId,
          x: 200,
          y: 100,
          width: 32,
          height: 32,
        },
      },
    ],
  });
  assert.equal(committed.committedRevision, 3);
  browser = await chromium.launch();
  const page = await browser.newPage({
    ignoreHTTPSErrors: true,
    viewport: { width: 1400, height: 1000 },
  });
  const errors = [],
    failed = [],
    requests = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("requestfailed", (request) => {
    if (request.failure()?.errorText !== "net::ERR_ABORTED")
      failed.push(request.url());
  });
  page.on("request", (request) =>
    requests.push({ url: request.url(), method: request.method() }),
  );
  await page.goto(`${server.origin}/?project=${projectId}&frame=frame`);
  await page.getByText("Scene r3", { exact: true }).waitFor();
  assert.ok(await page.locator("canvas").count());
  assert.equal(await page.getByText(/Missing font:|Missing asset:/).count(), 0);
  await page.getByRole("button", { name: "Fonts", exact: true }).click();
  const dialog = page.getByRole("dialog", { name: "Project fonts" });
  await dialog.getByText("Public Noto", { exact: true }).waitFor();
  await dialog.getByText(/Public viewer:/).waitFor();
  assert.equal(
    await dialog.getByLabel("Font file", { exact: true }).count(),
    0,
  );
  const [download] = await Promise.all([
    page.waitForEvent("download"),
    dialog
      .getByRole("button", { name: "Download Public Noto, 400 normal" })
      .click(),
  ]);
  const hash = (bytes) => createHash("sha256").update(bytes).digest("hex");
  assert.equal(hash(await readFile(await download.path())), hash(font));
  await dialog.getByRole("button", { name: "Close fonts" }).click();
  await page
    .getByRole("button", { name: "Public label", exact: true })
    .click({ button: "right" });
  const [exported] = await Promise.all([
    page.waitForEvent("download"),
    page.getByRole("menuitem", { name: "Export SVG", exact: true }).click(),
  ]);
  assert.match(
    await readFile(await exported.path(), "utf8"),
    /data:font\/ttf;base64,/,
  );
  const changed = await mcp.call("apply_batch", {
    projectId,
    expectedRevision: 3,
    idempotencyKey: "live",
    operations: [
      { op: "update_node", id: "text", props: { text: "Public live Việt" } },
    ],
  });
  assert.equal(changed.committedRevision, 4);
  await page.getByText("Scene r4", { exact: true }).waitFor();
  await page.screenshot({ path: join(artifacts, "viewer.png") });
  await page.goto(`${server.origin}/preview?project=${projectId}&frame=frame`);
  await page
    .locator('.oc-node[data-oc-id="text"]')
    .getByText("Public live Việt", { exact: true })
    .waitFor();
  const image = page.locator("img.oc-image");
  await image.waitFor();
  assert.equal(
    await image.evaluate(
      (element) => element.complete && element.naturalWidth > 0,
    ),
    true,
  );
  assert.ok((await image.getAttribute("src")).startsWith(server.origin));
  await page.screenshot({ path: join(artifacts, "preview.png") });
  assert.ok(
    requests.some((request) =>
      request.url.includes("/fonts/font_public/download"),
    ),
  );
  assert.ok(
    requests.some((request) => request.url.includes(`/assets/${assetId}`)),
  );
  assert.ok(requests.some((request) => request.url.includes("/events?")));
  assert.ok(
    requests.every(
      (request) =>
        request.url.startsWith(server.origin) ||
        request.url.startsWith("blob:"),
    ),
    JSON.stringify(requests),
  );
  assert.ok(
    requests.every(
      (request) =>
        request.method !== "POST" || request.url.endsWith("/exports"),
    ),
  );
  assert.deepEqual(errors, []);
  assert.deepEqual(failed, []);
  const final = await (
    await fetch(`${server.local}/api/v1/projects/${projectId}/snapshot`)
  ).json();
  assert.equal(
    final.revision,
    4,
    "Public downloads/export do not change revision",
  );
  await page.goto(`${server.local}/?project=${projectId}&frame=frame`);
  await page.getByRole("button", { name: "Fonts", exact: true }).click();
  await page
    .getByRole("dialog", { name: "Project fonts" })
    .getByLabel("Font file", { exact: true })
    .waitFor();
  console.log(
    JSON.stringify({
      passed: true,
      checks: [
        "HTTPS viewer",
        "public Fonts download only",
        "SVG download",
        "live SSE",
        "preview image/font",
        "all browser URLs same public origin",
        "local font import UI",
      ],
      artifacts,
    }),
  );
} finally {
  await browser?.close();
  await mcp?.close();
  await server?.stop();
  await rm(root, { recursive: true, force: true });
}
