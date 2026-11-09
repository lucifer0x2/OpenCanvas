import assert from "node:assert/strict";
import { performance } from "node:perf_hooks";

export const PROFILES = {
  short: Array(9).fill(600),
  medium: Array(9).fill(1800),
  long: Array(9).fill(6000),
  mixed: [600, 1200, 2400, 4800, 9600, 1800, 6000, 900, 3600],
};
const node = (id, kind, parentId, props) => ({
  op: "create_node",
  id,
  kind,
  parentId,
  props,
});

/** Exactly 10,000 renderable nodes, excluding the page container. */
export function operations(profile, heights = PROFILES[profile]) {
  const frames = heights.length;
  const result = [
    node("page", "page", null, { name: `${profile} / 10,000 nodes` }),
  ];
  for (let frame = 0; frame < frames; frame++) {
    const id = `frame-${frame}`,
      height = heights[frame];
    const count =
      Math.floor((10000 - frames) / frames) +
      (frame < (10000 - frames) % frames ? 1 : 0);
    result.push(
      node(id, "frame", "page", {
        name: `${profile} ${frame + 1} / 800 × ${height}`,
        width: 800,
        height,
        fill: "#ffffff",
      }),
    );
    const rows = Math.ceil(count / 32),
      cell = 752 / 32;
    for (let index = 0; index < count; index++) {
      const isText = index % 5 === 0;
      result.push(
        node(`${id}-n${index}`, isText ? "text" : "rectangle", id, {
          name: `${isText ? "Text" : "Tile"} ${index}`,
          x: 24 + (index % 32) * cell,
          y: 24 + Math.floor(index / 32) * ((height - 48) / rows),
          width: cell - 3,
          height: Math.min(14, (height - 48) / rows - 2),
          fill: isText
            ? "#26364a"
            : ["#dbe8ef", "#dfeadd", "#eee0ce"][index % 3],
          ...(isText
            ? {
                text: `n${index % 10}`,
                fontId: "font_bundled",
                fontSize: 8,
                lineHeight: 10,
              }
            : {}),
        }),
      );
    }
  }
  assert.equal(result.length, 10001);
  return result;
}

export async function seed(mcp, profile, heights) {
  const started = performance.now();
  const { projectId } = await mcp.call("create_project", {
    slug: `stress-${profile}`,
    name: `Stress ${profile} 10k`,
  });
  const ops = operations(profile, heights);
  let revision = 0;
  const batches = [];
  for (let offset = 0; offset < ops.length; offset += 500) {
    const start = performance.now();
    const response = await mcp.call("apply_batch", {
      projectId,
      expectedRevision: revision,
      idempotencyKey: `seed-${offset}`,
      operations: ops.slice(offset, offset + 500),
    });
    revision = response.committedRevision;
    batches.push(performance.now() - start);
  }
  return {
    profile,
    projectId,
    revision,
    heights: heights ?? PROFILES[profile],
    seedMs: performance.now() - started,
    batchesMs: batches,
  };
}
