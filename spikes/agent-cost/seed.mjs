// Project fixtures created through actual MCP mutations (never direct DB writes).
import assert from "node:assert/strict";
import { author } from "../../samples/welcome-stacks/author.mjs";
import { FRAME } from "../../samples/welcome-stacks/design.mjs";

export const STACKS_FRAME = FRAME;
export const STACKS_PAGE = "welcome-page-stacks";
/** Text node whose copy is "Tổng quan" in the stacks sample (S1 target). */
export const OVERVIEW_TEXT = "Tổng quan";

/** The Task02 stacks sample: one page, one 1040×720 frame, revision 1. */
export async function seedStacks(mcp, slug = "opencanvas-welcome-stacks") {
  const project = await author(mcp, slug);
  return {
    projectId: project.projectId,
    revision: project.revision,
    frameId: FRAME,
    nodeCount: project.nodeCount,
  };
}

/** A minimal 3-node project; S5 only needs the data root to hold more projects. */
export async function seedSmall(mcp, slug) {
  const project = await mcp.call("create_project", {
    slug,
    name: `Filler ${slug}`,
  });
  await mcp.call("apply_batch", {
    projectId: project.projectId,
    expectedRevision: 0,
    idempotencyKey: `${slug}-v1`,
    operations: [
      {
        op: "create_node",
        id: "page",
        kind: "page",
        parentId: null,
        props: { name: "Page" },
      },
      {
        op: "create_node",
        id: "frame",
        kind: "frame",
        parentId: "page",
        props: { name: "Frame", width: 320, height: 200 },
      },
      {
        op: "create_node",
        id: "label",
        kind: "text",
        parentId: "frame",
        props: {
          name: "label",
          text: "Xin chào",
          width: 200,
          height: "hug",
          fontId: "font_bundled",
          fontSize: 16,
          lineHeight: 24,
        },
      },
    ],
  });
  return project.projectId;
}

/**
 * Same shape as the "large" storage fixture (spikes/storage/README.md,
 * crates/storage/examples/spike/fixture.rs): one page, two fixed 1440×900
 * frames (column / row), alternating rectangle/text leaves, Vietnamese text,
 * a literal token and a primitive style. `size` counts every authored node.
 */
export function largeOperations(size) {
  const ops = [
    {
      op: "upsert_token",
      token: { id: "brand", name: "Brand", type: "color", value: "#1a73e8" },
    },
    {
      op: "upsert_style",
      style: {
        id: "surface",
        name: "Surface",
        kind: "fill",
        value: { tokenId: "brand" },
      },
    },
    {
      op: "create_node",
      id: "page",
      kind: "page",
      parentId: null,
      props: { name: "Storage fixture" },
    },
    {
      op: "create_node",
      id: "frame-a",
      kind: "frame",
      parentId: "page",
      props: {
        width: 1440,
        height: 900,
        layout: { mode: "column", gap: 8, padding: 24 },
      },
    },
    {
      op: "create_node",
      id: "frame-b",
      kind: "frame",
      parentId: "page",
      props: {
        width: 1440,
        height: 900,
        layout: { mode: "row", gap: 8, padding: 24 },
      },
    },
  ];
  for (let i = 0; i < size - 3; i++) {
    const text = i >= 10 && i % 3 === 0;
    const props = text
      ? {
          name: "base",
          text: "Thiết kế tiếng Việt: durable revision",
          width: 240,
          height: "hug",
          fontId: "font_bundled",
          fontSize: 16,
          lineHeight: 24,
          fill: { tokenId: "brand" },
        }
      : { name: "base", width: 80, height: 40, fill: { styleId: "surface" } };
    ops.push({
      op: "create_node",
      id: `leaf-${i}`,
      kind: text ? "text" : "rectangle",
      parentId: i % 2 === 0 ? "frame-a" : "frame-b",
      props,
    });
  }
  return ops;
}

/** Commits largeOperations in 500-operation batches (the apply_batch cap). */
export async function seedLarge(mcp, size, slug = "large-fixture") {
  const project = await mcp.call("create_project", {
    slug,
    name: "Large fixture",
  });
  const ops = largeOperations(size);
  let revision = 0;
  for (let i = 0; i < ops.length; i += 500) {
    const result = await mcp.call("apply_batch", {
      projectId: project.projectId,
      expectedRevision: revision,
      idempotencyKey: `seed-${i / 500}`,
      operations: ops.slice(i, i + 500),
    });
    revision = result.committedRevision;
  }
  assert.equal(ops.filter((o) => o.op === "create_node").length, size);
  return {
    projectId: project.projectId,
    revision,
    frameId: "frame-a",
    nodeCount: size,
  };
}
