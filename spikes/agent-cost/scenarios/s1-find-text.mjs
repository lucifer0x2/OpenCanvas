// S1 — find the text node whose copy is "Tổng quan" and read its props.
// The agent knows only the copy; outline names are node IDs, so text content is
// visible only through props reads.
import assert from "node:assert/strict";
import { withEnvironment, seedThenRestart } from "../env.mjs";
import { recorder } from "../meter.mjs";
import { seedStacks, OVERVIEW_TEXT } from "../seed.mjs";
import { readAll, summarize } from "./common.mjs";

export const id = "S1";
export const title = `Find the "${OVERVIEW_TEXT}" text node and read its props`;

const matches = (node) =>
  typeof node.props?.text === "string" &&
  node.props.text.includes(OVERVIEW_TEXT);

/**
 * EXTENSION POINT: add a strategy (e.g. `findNodes` once Task 06 exists) and
 * select it with `--opt s1.strategy=<name>`. Each returns {nodeId, props}.
 */
export const strategies = {
  /** Baseline, tools available today: list_projects → full outline → page subtrees until found. */
  async outlineThenSubtree(client, rec, opts) {
    rec.phase("discover");
    const { projects } = await client.call("list_projects", {});
    const projectId = projects[0].projectId;
    rec.phase("outline");
    const outline = (
      await readAll(client, "get_outline", { projectId })
    ).flatMap((p) => p.nodes);
    rec.phase("search");
    let subtreePages = 0;
    for (const page of outline.filter((n) => n.kind === "page")) {
      let cursor;
      do {
        const read = await client.call("get_subtree", {
          projectId,
          nodeId: page.id,
          ...opts.subtreeArgs,
          ...(cursor ? { cursor } : {}),
        });
        subtreePages++;
        const hit = read.nodes.find(matches);
        // Props arrive with the subtree page, so the search read is also the props read.
        if (hit)
          return {
            projectId,
            nodeId: hit.id,
            props: hit.props,
            outlineNodes: outline.length,
            subtreePages,
          };
        cursor = read.nextCursor;
      } while (cursor);
    }
    throw new Error(`No node contains "${OVERVIEW_TEXT}"`);
  },
  /** Task 06: list_projects → find_nodes by copy → get_subtree depth 0 for the node's full props. */
  async findNodes(client, rec, opts) {
    rec.phase("discover");
    const { projects } = await client.call("list_projects", {});
    const projectId = projects[0].projectId;
    rec.phase("search");
    const found = await client.call("find_nodes", {
      projectId,
      textContains: OVERVIEW_TEXT,
      limit: 1,
    });
    const hit = found.matches[0];
    if (!hit) throw new Error(`No node contains "${OVERVIEW_TEXT}"`);
    rec.phase("props");
    const read = await client.call("get_subtree", {
      projectId,
      nodeId: hit.id,
      depth: 0,
      ...opts.subtreeArgs,
    });
    return {
      projectId,
      nodeId: hit.id,
      props: read.nodes[0].props,
      outlineNodes: 0,
      subtreePages: 1,
      totalMatches: found.totalMatches,
      scannedNodes: found.scannedNodes,
    };
  },
};

export async function run(meter, options) {
  const opts = options.s1;
  const strategy = strategies[opts.strategy];
  assert.ok(strategy, `Unknown S1 strategy ${opts.strategy}`);
  const { result, cleanup } = await withEnvironment(async (env) => {
    await seedThenRestart(env, (mcp) => seedStacks(mcp));
    const rec = recorder(meter);
    const client = rec.client(await env.connect());
    const found = await strategy(client, rec, opts);
    assert.ok(
      matches({ props: found.props }),
      "Found props must contain the target copy",
    );
    return summarize(id, title, rec, {
      strategy: opts.strategy,
      nodeId: found.nodeId,
      propKeys: Object.keys(found.props).sort(),
      outlineNodes: found.outlineNodes,
      subtreePages: found.subtreePages,
    });
  });
  return { ...result, cleanup };
}
