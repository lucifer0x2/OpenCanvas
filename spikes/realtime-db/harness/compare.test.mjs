import { test } from 'node:test';
import assert from 'node:assert/strict';
import { comparison } from './compare.mjs';

function reports() {
  const matrix = [100, 5000].flatMap(nodeCount => [1, 4, 8].map(writers => ({
    nodeCount, writers, iterations: 12, successfulCount: writers * 12, requestAttempts: writers * 12,
    readCount: 1, observedCommits: writers * 12, attemptMs: {}, retryToSuccessMs: {},
  })));
  const baseline = { status: 'pass', startedAt: '2026-11-02T01:00:00Z', finishedAt: '2026-11-02T01:01:00Z',
    cleanup: 'owned resources removed', configuration: { nodeCounts: '100,5000', writers: '1,4,8', iterations: 12 },
    topology: { image: 'fixed', imageId: 'same', cpu: 2, memoryBytes: 2147483648, postgresVersion: '16.9', node: '24', gatewayHost: {} },
    provenance: { sourceFingerprint: 'before', rustc: 'same', cargo: 'same' }, provenanceAfter: { sourceFingerprint: 'before' },
    phases: { contractAndMatrix: { status: 'pass', result: { matrix } } } };
  const optimized = structuredClone(baseline);
  optimized.startedAt = '2026-11-02T01:02:00Z';
  optimized.phases.contractAndMatrix.result.matrix = ['legacy', 'optimized'].flatMap(clientMode => matrix.map(value => ({ ...value, clientMode })));
  return [baseline, optimized];
}

test('comparison requires complete real traffic in all three modes', () => {
  assert.equal(comparison(...reports()).rows.length, 6);
  const [base, optimized] = reports();
  optimized.phases.contractAndMatrix.result.matrix.pop();
  assert.throws(() => comparison(base, optimized), /Missing/);
  const [empty, complete] = reports();
  empty.phases.contractAndMatrix.result.matrix[0].readCount = 0;
  assert.throws(() => comparison(empty, complete));
});

test('comparison refuses changed resources, overlapping runs and source mutation', () => {
  for (const mutate of [
    value => { value.topology.cpu = 4; },
    value => { value.startedAt = '2026-11-02T01:00:30Z'; },
    value => { value.provenanceAfter.sourceFingerprint = 'changed'; },
  ]) {
    const [base, optimized] = reports(); mutate(optimized);
    assert.throws(() => comparison(base, optimized));
  }
});
