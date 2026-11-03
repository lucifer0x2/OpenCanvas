import assert from 'node:assert/strict';
import { fixture } from './load.mjs';
import { request } from './client.mjs';
import { cacheRead, metricDelta, metrics } from './task17-cache-assertions.mjs';
import { quantiles } from './single-writer-load.mjs';

// Reader-only counterfactual uses the identical binary and PG resources, toggling Redis.
export async function hotReadLoad(urls, nodeCounts = [100, 5000]) {
  const cells = [];
  for (const nodeCount of nodeCounts) {
    const { projectId, revision } = await fixture(urls[0], nodeCount);
    for (let n = 0; n < 20; n++) await cacheRead(urls[n % 2], projectId, revision);
    const before = await metrics(urls), raw = [];
    for (let n = 0; n < 100; n++) {
      const result = await request(urls[n % 2], `/projects/${projectId}`);
      assert.equal(result.status, 200); assert.equal(result.body.revision, revision);
      assert.equal(result.body.nodes.length, nodeCount); raw.push(result.ms);
    }
    const delta = metricDelta(before, await metrics(urls));
    const cached = delta.disabled === 0;
    assert.equal(delta.nodeQueries, cached ? 0 : 100);
    assert.equal(delta.hits, cached ? 100 : 0);
    cells.push({ nodeCount, writers: 0, warmup: 20, samples: 100, latencyMs: quantiles(raw), metrics: delta, rawMs: raw });
  }
  return cells;
}
