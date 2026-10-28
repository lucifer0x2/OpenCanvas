// S2 — read the complete design context of the stacks frame (every page).
import assert from "node:assert/strict";
import { withEnvironment, seedThenRestart } from "../env.mjs";
import { recorder } from "../meter.mjs";
import { seedStacks } from "../seed.mjs";
import { nodesOf, readAll, summarize } from "./common.mjs";

export const id = "S2";
export const title =
  "Full get_design_context of the stacks frame, following nextCursor";

const RESOURCES = ["tokens", "styles", "assets", "fonts"];

export async function run(meter, options) {
  const opts = options.s2;
  const { result, cleanup } = await withEnvironment(async (env) => {
    const project = await seedThenRestart(env, (mcp) => seedStacks(mcp));
    const rec = recorder(meter);
    const client = rec.client(await env.connect());
    rec.phase("context");
    // EXTENSION POINT: s2.contextArgs (e.g. {"compact":true}) is merged into every page request.
    const pages = await readAll(client, "get_design_context", {
      projectId: project.projectId,
      frameId: project.frameId,
      ...opts.contextArgs,
    });
    const { nodes, count, unique } = nodesOf(pages);
    assert.equal(count, unique, "Context pages must not repeat nodes");
    assert.ok(count > 0, "Context returned no nodes");
    const resources = Object.fromEntries(
      RESOURCES.map((key) => [
        key,
        pages.reduce((sum, p) => sum + (p[key]?.length ?? 0), 0),
      ]),
    );
    const resolvedPropsBytes = nodes.reduce(
      (sum, n) =>
        sum +
        (n.resolvedProps
          ? Buffer.byteLength(JSON.stringify(n.resolvedProps))
          : 0),
      0,
    );
    return summarize(id, title, rec, {
      contextArgs: opts.contextArgs,
      pages: pages.length,
      nodes: count,
      authoredNodes: project.nodeCount,
      resources,
      resolvedPropsBytes,
      pageResponseBytes: rec.calls.map((c) => c.responseBytes),
    });
  });
  return { ...result, cleanup };
}
