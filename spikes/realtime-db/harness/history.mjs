import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';
import { batch, request, update } from './client.mjs';
import { fixture } from './load.mjs';
import { databaseStats, restartEngine } from '../infra/runtime.mjs';

export async function history(runtime, gateways, count = 1000) {
  const { projectId, revision } = await fixture(gateways.urls[0], 5000);
  const samples = [{ addedCommits: 0, ...databaseStats(runtime) }];
  for (let n = 0; n < count; n++) {
    const result = await request(gateways.urls[n % 2], '/batch', batch(projectId, revision + n,
      [update('sentinel-a', { name: String(n) }), update('sentinel-b', { name: String(n) })]));
    assert.equal(result.status, 200, JSON.stringify(result.body));
    if (n + 1 === Math.floor(count / 4) || n + 1 === count) samples.push({ addedCommits: n + 1, ...databaseStats(runtime) });
  }
  const start = performance.now();
  const restart = await gateways.restart(1, 0);
  const state = await request(gateways.urls[1], `/projects/${projectId}`);
  assert.equal(state.status, 200); assert.equal(state.body.revision, revision + count);
  assert.equal(state.body.nodes.length, 5000);
  const gatewayRestartAndSnapshotMs = performance.now() - start;
  const databaseStart = performance.now();
  await restartEngine(runtime, 100);
  const recovered = await request(gateways.urls[0], `/projects/${projectId}`);
  assert.equal(recovered.status, 200); assert.equal(recovered.body.revision, revision + count);
  assert.deepEqual(recovered.body, state.body);
  const commits = (await request(gateways.urls[1], `/changes/${projectId}?sinceRevision=${revision}`)).body.commits;
  assert.equal(commits.length, count);
  assert.deepEqual(commits.map(value => value.revision), Array.from({ length: count }, (_, index) => revision + index + 1));
  return { projectId, fixtureNodes: 5000, additionalHistoryCommits: count, samples, restart,
    gatewayRestartAndSnapshotMs, databaseRestartAndSnapshotMs: performance.now() - databaseStart,
    replayAfterDatabaseRestart: commits.length, afterRestart: databaseStats(runtime),
    scope: 'Synthetic bounded growth only; no retention/GC or production capacity/SLA inference' };
}
