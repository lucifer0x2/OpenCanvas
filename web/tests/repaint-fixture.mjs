// Temporary native backend and a realistic 12-frame page for all-frames-repaint.mjs.
// SQLite data root in the OS temp directory, ephemeral loopback port; removed by stop().
import { spawn } from "node:child_process";
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { connectMcp, waitFor } from "../../spikes/e2e/protocol.mjs";
import { stripesPNG } from "./fixture-png.mjs";

const ROOT = fileURLToPath(new URL("../../", import.meta.url));
export const HEIGHTS = [2400, 3200, 1800, 4000, 2600, 3000, 2200, 3600, 2000, 2800, 3400, 2400];
/** The frame and text node every edit touches. */
export const EDITED = { frame: "frame-5", node: "frame-5-s3-c2-title" };

export async function startBackend() {
  const dataRoot = await mkdtemp(join(tmpdir(), "opencanvas-repaint-"));
  const binary =
    process.env.REPAINT_SERVER_BIN ?? join(ROOT, "target/debug/opencanvas-server");
  // A clean environment: never inherit a live database URL, port or public origin.
  const env = {
    PATH: process.env.PATH,
    HOME: process.env.HOME,
    TMPDIR: process.env.TMPDIR ?? "",
    DATABASE_PROVIDER: "sqlite",
    RENDER_BACKEND: "rust",
  };
  const child = spawn(
    binary,
    ["--data-root", dataRoot, "--port", "0", "--workspace", ROOT],
    { cwd: ROOT, detached: true, stdio: ["ignore", "pipe", "pipe"], env },
  );
  let output = "";
  child.stdout.on("data", (bytes) => (output += bytes));
  child.stderr.on("data", (bytes) => (output += bytes));
  const base = await waitFor(() => {
    if (child.exitCode !== null) throw new Error(output);
    return output.match(/OpenCanvas (http:\/\/127\.0\.0\.1:\d+)/)?.[1];
  }, "temporary backend", 15_000);
  return {
    base,
    dataRoot,
    async stop() {
      if (child.exitCode === null) {
        process.kill(-child.pid, "SIGTERM");
        await new Promise((resolve) => child.once("exit", resolve));
      }
      await rm(dataRoot, { recursive: true, force: true });
    },
  };
}

const node = (id, kind, parentId, props) => ({ op: "create_node", id, kind, parentId, props });
const text = (id, parentId, x, y, width, size, value, fill = "#1d2433") =>
  node(id, "text", parentId, {
    name: id, text: value, x, y, width, height: "hug",
    fontId: "font_bundled", fontSize: size, lineHeight: Math.round(size * 1.4), fill,
  });
const COPY =
  "Plan, review and ship design changes with live previews, shared tokens and precise layout.";

/** One landing-page-like frame: ten sections of eight cards, about 500 nodes. */
function frameOps(index, height, assetId) {
  const id = `frame-${index}`;
  const ops = [
    node(id, "frame", "page", { name: `Screen ${index + 1}`, width: 1440, height, fill: "#ffffff" }),
    text(`${id}-hero`, id, 96, 48, 900, 40, `Screen ${index + 1} — product overview`),
  ];
  const sectionHeight = (height - 160) / 10;
  for (let s = 0; s < 10; s++) {
    const section = `${id}-s${s}`;
    ops.push(node(section, "frame", id, {
      name: `Section ${s}`, x: 48, y: 140 + s * sectionHeight, width: 1344,
      height: sectionHeight - 16, radius: 16, fill: s % 2 ? "#f6f8fb" : "#eef3f8",
      stroke: { color: "#d9dfe8", width: 1 },
    }));
    for (let c = 0; c < 8; c++) {
      const card = `${section}-c${c}`;
      const w = 152, h = Math.min(220, sectionHeight - 48);
      ops.push(node(card, "frame", section, {
        name: `Card ${c}`, x: 16 + c * 166, y: 16, width: w, height: h, radius: 12, fill: "#ffffff",
        stroke: { color: "#c9d3e0", width: 1 },
        shadow: [{ x: 0, y: 4, blur: 12, color: "#1d2433" }],
        opacity: c === 7 ? 0.8 : 1,
      }));
      ops.push(
        c % 3 === 0
          ? node(`${card}-media`, "image", card, { name: "Media", x: 8, y: 8, width: w - 16, height: 56, radius: 8, assetId, fit: "cover" })
          : node(`${card}-media`, "rectangle", card, { name: "Media", x: 8, y: 8, width: w - 16, height: 56, radius: 8, fill: ["#dbe8ef", "#dfeadd", "#eee0ce"][c % 3] }),
        text(`${card}-title`, card, 10, 72, w - 20, 14, `Card ${s}.${c} title`),
        text(`${card}-body`, card, 10, 96, w - 20, 11, COPY, "#505b6b"),
        node(`${card}-cta`, "rectangle", card, {
          name: "Button", x: 10, y: h - 34, width: 80, height: 24, radius: 6, fill: "#1672e8",
          stroke: { color: "#0d4fa8", width: 1, align: "outside" },
        }),
      );
    }
  }
  return ops;
}

/** Create the project through MCP and return what the browser and edits need. */
export async function seedProject(base) {
  const mcp = await connectMcp(base);
  const { projectId } = await mcp.call("create_project", { slug: "repaint", name: "Repaint fixture" });
  const imported = await mcp.call("import_asset", {
    projectId, expectedRevision: 0, idempotencyKey: "image", mediaType: "image/png",
    base64: stripesPNG(320, 120).toString("base64"),
  });
  let revision = imported.committedRevision ?? 1;
  const ops = [node("page", "page", null, { name: "Board" })];
  HEIGHTS.forEach((height, i) => ops.push(...frameOps(i, height, imported.asset.id)));
  for (let offset = 0; offset < ops.length; offset += 500) {
    const result = await mcp.call("apply_batch", {
      projectId, expectedRevision: revision, idempotencyKey: `seed-${offset}`,
      operations: ops.slice(offset, offset + 500),
    });
    revision = result.committedRevision;
  }
  let edits = 0;
  return {
    projectId,
    get revision() {
      return revision;
    },
    nodes: ops.length,
    /** One text change in one frame; resolves with the committed revision. */
    async edit() {
      edits++;
      const result = await mcp.call("apply_batch", {
        projectId, expectedRevision: revision, idempotencyKey: `edit-${edits}`,
        operations: [{ op: "update_node", id: EDITED.node, props: { text: `Edited title ${edits}` } }],
      });
      revision = result.committedRevision;
      return revision;
    },
    close: () => mcp.close(),
  };
}
