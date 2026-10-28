// S6 — read a generated 5,000-node project shaped like the storage spike's
// "large" fixture: complete outline, then complete design context of frame-a.
import assert from "node:assert/strict";
import { withEnvironment, seedThenRestart } from "../env.mjs";
import { latency, recorder } from "../meter.mjs";
import { seedLarge } from "../seed.mjs";
import { nodesOf, readAll, summarize } from "./common.mjs";

export const id = "S6";
export const title =
  "5,000-node project: full get_outline and full frame get_design_context";

export async function run(meter, options) {
  const opts = options.s6;
  const { result, cleanup } = await withEnvironment(async (env) => {
    const project = await seedThenRestart(env, (mcp) =>
      seedLarge(mcp, opts.nodes),
    );
    const { projectId, frameId } = project;
    const rec = recorder(meter);
    const client = rec.client(await env.connect());
    // EXTENSION POINT: s6.outlineArgs / s6.contextArgs are merged into every page request.
    rec.phase("outline");
    const outline = nodesOf(
      await readAll(client, "get_outline", { projectId, ...opts.outlineArgs }),
    );
    assert.equal(
      outline.count,
      opts.nodes,
      "Outline must list every authored node",
    );
    assert.equal(outline.unique, outline.count);
    rec.phase("context");
    const context = nodesOf(
      await readAll(client, "get_design_context", {
        projectId,
        frameId,
        ...opts.contextArgs,
      }),
    );
    assert.equal(context.unique, context.count);
    const pick = (phase) => rec.calls.filter((c) => c.phase === phase);
    return summarize(id, title, rec, {
      authoredNodes: opts.nodes,
      outline: {
        pages: pick("outline").length,
        nodes: outline.count,
        latency: latency(pick("outline")),
      },
      context: {
        pages: pick("context").length,
        nodes: context.count,
        latency: latency(pick("context")),
      },
    });
  });
  return { ...result, cleanup };
}
