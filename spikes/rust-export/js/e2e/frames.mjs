// Part D frames, authored through real MCP create_project + apply_batch on a temp server:
// the welcome-stacks sample, the original free-layout welcome sample, and a synthetic 2×2 grid
// of the stacks dashboard (each copy's text suffixed so its measurements are not shared).
import { rmSync } from "node:fs";
import { connectMcp } from "../../../e2e/protocol.mjs";
import { welcomeOperations, FRAME as WELCOME_FRAME } from "../../../../samples/welcome/design.mjs";
import { stacksOperations, FRAME as STACKS_FRAME } from "../../../../samples/welcome-stacks/design.mjs";
import { AUTHORED, startServer, writeJson, work } from "./common.mjs";

export const GRID_FRAME = "bench-grid";
const GRID = { copies: 4, columns: 2, width: 1040, height: 720 };

/** name → project slug, frame, base viewport. Warm runs widen the viewport 1 px per job. */
export const FRAMES = [
  { name: "stacks-1040", slug: "stacks", frameId: STACKS_FRAME, viewport: { width: 1040, height: 720 } },
  { name: "stacks-390", slug: "stacks", frameId: STACKS_FRAME, viewport: { width: 390, height: 844 } },
  { name: "welcome-1040", slug: "welcome", frameId: WELCOME_FRAME, viewport: { width: 1040, height: 720 } },
  { name: "grid-2080", slug: "grid", frameId: GRID_FRAME, viewport: { width: 2080, height: 1440 } },
];

export function gridOperations() {
  const source = stacksOperations();
  const frame = source.find((o) => o.id === STACKS_FRAME);
  const ops = source.filter((o) => o.op !== "create_node"); // tokens and styles
  const node = (id, kind, parentId, props) => ops.push({ op: "create_node", id, kind, parentId, props: { name: id, ...props } });
  node("bench-page", "page", null, {});
  node(GRID_FRAME, "frame", "bench-page", {
    width: GRID.width * GRID.columns, height: GRID.height * Math.ceil(GRID.copies / GRID.columns),
    clip: true, fill: "#FFFFFF", layout: { mode: "column", padding: 0, gap: 0 },
  });
  for (let row = 0; row < Math.ceil(GRID.copies / GRID.columns); row++)
    node(`grid-row-${row}`, "group", GRID_FRAME, { width: "fill", height: GRID.height, layout: { mode: "row", padding: 0, gap: 0 } });
  for (let k = 0; k < GRID.copies; k++) {
    const copy = `c${k}`;
    node(copy, "group", `grid-row-${Math.floor(k / GRID.columns)}`, { ...frame.props, name: copy, width: GRID.width, height: GRID.height });
    for (const o of source) {
      if (o.op !== "create_node" || o.kind === "page" || o.id === STACKS_FRAME) continue;
      const props = { ...o.props };
      if (o.kind === "text" && k > 0) props.text = `${props.text} ${k + 1}`;
      ops.push({ ...o, id: `${copy}-${o.id}`, parentId: o.parentId === STACKS_FRAME ? copy : `${copy}-${o.parentId}`, props });
    }
  }
  return ops;
}

const OPERATIONS = { stacks: stacksOperations, welcome: welcomeOperations, grid: gridOperations };

/** Authors the three projects into AUTHORED and records ids, node and text counts. */
export async function authorFrames() {
  rmSync(AUTHORED, { recursive: true, force: true });
  const server = await startServer(AUTHORED);
  const mcp = await connectMcp(server.base);
  const projects = {};
  try {
    for (const [slug, make] of Object.entries(OPERATIONS)) {
      const operations = make();
      const project = await mcp.call("create_project", { slug: `bench-${slug}`, name: `Bench ${slug}` });
      let revision = 0; // apply_batch takes at most 500 operations; parents precede children
      for (let i = 0; i < operations.length; i += 500) {
        const result = await mcp.call("apply_batch", { projectId: project.projectId, expectedRevision: revision,
          idempotencyKey: `bench-${slug}-${i}`, operations: operations.slice(i, i + 500) });
        revision = result.committedRevision;
      }
      const nodes = operations.filter((o) => o.op === "create_node");
      projects[slug] = { projectId: project.projectId, revision, authoredNodes: nodes.length,
        textNodes: nodes.filter((o) => o.kind === "text").length };
    }
  } finally {
    await mcp.close();
    await server.stop();
  }
  writeJson(work("projects.json"), projects);
  return projects;
}
