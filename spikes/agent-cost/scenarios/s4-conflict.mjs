// S4 — two clients edit different nodes from the same base revision. Client A
// commits first; client B receives REVISION_CONFLICT, recovers and commits.
import assert from "node:assert/strict";
import { withEnvironment, seedThenRestart } from "../env.mjs";
import { recorder } from "../meter.mjs";
import { seedStacks } from "../seed.mjs";
import { readAll, summarize } from "./common.mjs";

export const id = "S4";
export const title =
  "Two clients on one base revision; the loser recovers until it commits";

const EDITS = {
  A: { nodeId: "overview-nav", props: { text: "Tổng quan · A" } },
  B: { nodeId: "goal-label", props: { text: "Mục tiêu tuần · B" } },
};

/**
 * EXTENSION POINT: recovery strategies receive the conflict error value and
 * return the revision to retry on. Add e.g. `getChanges` for Task 07D and select
 * it with `--opt s4.strategy=<name>`.
 */
export const strategies = {
  /** Baseline: the conflict carries only currentRevision, so re-read outline and context. */
  async rereadOutlineAndContext(client, { projectId, frameId }) {
    const outline = await readAll(client, "get_outline", { projectId });
    const context = await readAll(client, "get_design_context", {
      projectId,
      frameId,
    });
    assert.ok(
      outline.flatMap((p) => p.nodes).some((n) => n.id === EDITS.B.nodeId),
    );
    return context[0].revision;
  },
  /**
   * Task 07D: the conflict carries changedSince. Learn the edited node's ancestors with one
   * find_nodes path read (S4's base read is get_subtree depth 0, which has none), then retry on
   * toRevision when no changed ID is the node or an ancestor. Otherwise re-read the overlapping
   * IDs; an incomplete list falls back to the baseline readback.
   */
  async changedSince(client, args) {
    const since = args.conflict.error?.changedSince;
    assert.ok(since, "REVISION_CONFLICT must carry changedSince");
    const { projectId, edit } = args;
    const found = await client.call("find_nodes", {
      projectId,
      ids: [edit.nodeId],
    });
    const match = found.matches[0];
    if (since.truncated || !match || match.pathTruncated)
      return strategies.rereadOutlineAndContext(client, args);
    const mine = new Set([edit.nodeId, ...match.path.map((n) => n.id)]);
    const overlap = since.changedIds.filter((id) => mine.has(id));
    if (overlap.length > 0)
      await client.call("find_nodes", { projectId, ids: overlap });
    return since.toRevision;
  },
};

const batch = (projectId, revision, key, edit) => ({
  projectId,
  expectedRevision: revision,
  idempotencyKey: key,
  operations: [{ op: "update_node", id: edit.nodeId, props: edit.props }],
});

export async function run(meter, options) {
  const opts = options.s4;
  const recover = strategies[opts.strategy];
  assert.ok(recover, `Unknown S4 strategy ${opts.strategy}`);
  const { result, cleanup } = await withEnvironment(async (env) => {
    const project = await seedThenRestart(env, (mcp) => seedStacks(mcp));
    const { projectId, frameId } = project;
    const rec = recorder(meter);
    const a = rec.client(await env.connect(), "A");
    const b = rec.client(await env.connect(), "B");
    rec.phase("base");
    const baseA = await a.call("get_subtree", {
      projectId,
      nodeId: EDITS.A.nodeId,
      depth: 0,
    });
    const baseB = await b.call("get_subtree", {
      projectId,
      nodeId: EDITS.B.nodeId,
      depth: 0,
    });
    assert.equal(
      baseA.revision,
      baseB.revision,
      "Both clients must share the base revision",
    );
    rec.phase("commit");
    const winner = await a.call(
      "apply_batch",
      batch(projectId, baseA.revision, "s4-a-1", EDITS.A),
    );
    const conflict = await b.call(
      "apply_batch",
      batch(projectId, baseB.revision, "s4-b-1", EDITS.B),
      "REVISION_CONFLICT",
    );
    rec.phase("recover");
    const before = rec.calls.length;
    let attempts = 1;
    const retryRevision = await recover(b, {
      projectId,
      frameId,
      conflict,
      edit: EDITS.B,
    });
    // A changed body needs a new idempotency key.
    const committed = await b.call(
      "apply_batch",
      batch(projectId, retryRevision, `s4-b-${++attempts}`, EDITS.B),
    );
    assert.equal(committed.committedRevision, winner.committedRevision + 1);
    const recovery = rec.calls.slice(before);
    return summarize(id, title, rec, {
      strategy: opts.strategy,
      baseRevision: baseA.revision,
      winnerRevision: winner.committedRevision,
      conflictError: Object.keys(conflict.error ?? {}).sort(),
      loserRevision: committed.committedRevision,
      attempts,
      recoveryCalls: recovery.length,
      recoveryBytes: recovery.reduce(
        (sum, c) => sum + c.requestBytes + c.responseBytes,
        0,
      ),
    });
  });
  return { ...result, cleanup };
}
