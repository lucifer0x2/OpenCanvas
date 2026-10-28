// P03b AC7: an SVG export rasterized by Chromium matches the PNG export of the same node.
// Usage: node spikes/e2e/export-svg.mjs (after `npm run build:renderer` and a debug server build).
// Starts a temporary server on its own data root, exports one text-free frame at 1x PNG and SVG,
// draws the SVG through <img> and counts pixels whose channels differ by more than 16.
import assert from "node:assert/strict";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { connectMcp } from "./protocol.mjs";
import { startServer, workspace } from "./server.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= join(workspace, ".runtime/browsers");
const { chromium } = await import("playwright");
const MAX_DIFF_RATIO = 0.005;

function operations(assetId) {
  const node = (id, kind, parentId, props) => ({
    op: "create_node",
    id,
    kind,
    parentId,
    props,
  });
  return [
    node("page", "page", null, { name: "Home" }),
    node("frame", "frame", "page", {
      name: "Card",
      width: 240,
      height: 180,
      fill: "#f8fafc",
      radius: 16,
      layout: { mode: "free" },
    }),
    node("band", "rectangle", "frame", {
      x: 0,
      y: 0,
      width: 240,
      height: 48,
      fill: "#2563eb",
      opacity: 0.5,
    }),
    node("clipper", "group", "frame", {
      x: 16,
      y: 64,
      width: 96,
      height: 96,
      clip: true,
      radius: 12,
      opacity: 0.6,
      layout: { mode: "free" },
    }),
    node("under", "rectangle", "clipper", {
      x: -10,
      y: -10,
      width: 80,
      height: 80,
      fill: "#ef4444",
    }),
    node("over", "rectangle", "clipper", {
      x: 30,
      y: 30,
      width: 80,
      height: 80,
      fill: "#11223380",
      radius: 6,
    }),
    node("photo", "image", "frame", {
      x: 128,
      y: 64,
      width: 96,
      height: 96,
      assetId,
      fit: "cover",
      radius: 12,
      opacity: 0.6,
      crop: { x: 0.1, y: 0.2, width: 0.6, height: 0.5 },
    }),
  ];
}

async function gradientPng(browser) {
  const page = await browser.newPage();
  const data = await page.evaluate(() => {
    const canvas = document.createElement("canvas");
    canvas.width = 64;
    canvas.height = 40;
    const ctx = canvas.getContext("2d");
    const g = ctx.createLinearGradient(0, 0, 64, 40);
    g.addColorStop(0, "#0ea5e9");
    g.addColorStop(1, "#f59e0b");
    ctx.fillStyle = g;
    ctx.fillRect(0, 0, 64, 40);
    ctx.fillStyle = "#111827";
    ctx.fillRect(20, 10, 12, 12);
    return canvas.toDataURL("image/png").split(",")[1];
  });
  await page.close();
  return data;
}

async function download(file) {
  const response = await fetch(file.url);
  assert.equal(response.status, 200);
  assert.equal(response.headers.get("content-type"), file.mimeType);
  return Buffer.from(await response.arrayBuffer());
}

/** Draws both files at the PNG's size and counts pixels off by more than 16 in any channel. */
async function compare(browser, png, svg) {
  const page = await browser.newPage({ deviceScaleFactor: 1 });
  const result = await page.evaluate(
    async ({ png, svg }) => {
      async function pixels(src, size) {
        const image = new Image();
        image.src = src;
        await image.decode();
        const width = size?.width ?? image.naturalWidth;
        const height = size?.height ?? image.naturalHeight;
        const canvas = document.createElement("canvas");
        canvas.width = width;
        canvas.height = height;
        const ctx = canvas.getContext("2d");
        ctx.drawImage(image, 0, 0, width, height);
        return {
          width,
          height,
          data: ctx.getImageData(0, 0, width, height).data,
        };
      }
      const a = await pixels(`data:image/png;base64,${png}`);
      const b = await pixels(`data:image/svg+xml;base64,${svg}`, a);
      let off = 0;
      for (let i = 0; i < a.data.length; i += 4) {
        for (let c = 0; c < 4; c++) {
          if (Math.abs(a.data[i + c] - b.data[i + c]) > 16) {
            off++;
            break;
          }
        }
      }
      return {
        width: a.width,
        height: a.height,
        off,
        total: a.width * a.height,
      };
    },
    { png: png.toString("base64"), svg: svg.toString("base64") },
  );
  await page.close();
  return result;
}

const dataRoot = await mkdtemp(join(tmpdir(), "opencanvas-export-svg-"));
let server, mcp, browser;
try {
  server = await startServer(dataRoot);
  mcp = await connectMcp(server.base);
  browser = await chromium.launch({ headless: true });
  const { projectId } = await mcp.call("create_project", {
    slug: "e2e-export-svg",
    name: "Temporary SVG export",
  });
  const imported = await mcp.call("import_asset", {
    projectId,
    expectedRevision: 0,
    idempotencyKey: "svg-image",
    mediaType: "image/png",
    base64: await gradientPng(browser),
  });
  await mcp.call("apply_batch", {
    projectId,
    expectedRevision: imported.committedRevision,
    idempotencyKey: "svg-design",
    operations: operations(imported.asset.id),
  });
  const exported = await mcp.call("export_nodes", {
    projectId,
    nodeIds: ["frame"],
    settings: [{ scale: 1 }, { format: "svg" }],
  });
  const item = exported.exports[0];
  assert.equal(item.status, "ready", JSON.stringify(item));
  const [pngFile, svgFile] = item.files;
  assert.equal(svgFile.mimeType, "image/svg+xml");
  assert.equal(svgFile.diagnostics, undefined, "no text, no diagnostic");
  const svg = await download(svgFile);
  assert.ok(
    !svg.toString().includes(server.base),
    "SVG must not point back at the server",
  );
  const result = await compare(browser, await download(pngFile), svg);
  assert.deepEqual(
    [result.width, result.height],
    [pngFile.width, pngFile.height],
  );
  const ratio = result.off / result.total;
  console.log(JSON.stringify({ ...result, ratio, svgBytes: svg.length }));
  assert.ok(
    ratio <= MAX_DIFF_RATIO,
    `${(ratio * 100).toFixed(3)}% pixels differ by > 16`,
  );
} finally {
  await mcp?.close().catch(() => {});
  await browser?.close();
  await server?.stop();
  await rm(dataRoot, { recursive: true, force: true });
}
