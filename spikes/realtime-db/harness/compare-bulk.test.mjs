import assert from 'node:assert/strict';
import { test } from 'node:test';
import { compareBulk } from './compare-bulk.mjs';

function fixture(after = false) {
  const provenance = { sourceFingerprint: after ? 'optimized' : 'baseline', binaries: [{ path: 'gateway', sha256: after ? 'b' : 'a' }], rustc: 'rustc pinned', cargo: 'cargo pinned' };
  return { status: 'pass', startedAt: after ? '2026-11-02T01:00:00Z' : '2026-11-02T00:00:00Z',
    finishedAt: after ? '2026-11-02T01:01:00Z' : '2026-11-02T00:01:00Z', cleanup: 'owned resources removed',
    configuration: { nodeCounts: '100,5000', writers: '1,4,8', iterations: 12, clientModes: 'optimized', buildProfile: 'cargo dev' },
    provenance, provenanceAfter: structuredClone(provenance),
    topology: { containerId: after ? 'new' : 'old', image: 'pinned', imageId: 'sha256:fixed', cpu: 2, memoryBytes: 2147483648,
      postgresVersion: '16.9', node: 'v22', gatewayHost: { logicalCpus: 10 } },
    phases: { contractAndMatrix: { status: 'pass', result: { matrix: [100, 5000].flatMap(nodeCount => [1, 4, 8].map(writers => ({
      clientMode: 'optimized', nodeCount, writers, iterations: 12, successfulCount: writers * 12, observedCommits: writers * 12,
      readCount: 1, mixedSnapshots: 0, fairness: Array.from({ length: writers }, (_, writer) => ({ writer, successfulCount: 12 })),
    }))) } } } };
}

test('bulk comparison accepts exactly six same-client cells and independent DBs', () => {
  assert.equal(compareBulk(fixture(), fixture(true)).rows.length, 6);
});

test('bulk comparison rejects changed resources, profiles, clients and reused DBs', () => {
  for (const change of [value => { value.topology.cpu = 1; }, value => { value.configuration.buildProfile = 'release'; },
    value => { value.configuration.clientModes = 'legacy'; }, value => { value.topology.containerId = 'old'; }]) {
    const next = fixture(true); change(next); assert.throws(() => compareBulk(fixture(), next));
  }
});

test('bulk comparison rejects incomplete traffic, readers, fairness and mutated binaries', () => {
  for (const change of [value => value.phases.contractAndMatrix.result.matrix.pop(),
    value => { value.phases.contractAndMatrix.result.matrix[0].readCount = 0; },
    value => { value.phases.contractAndMatrix.result.matrix[0].fairness[0].successfulCount = 0; },
    value => { value.provenanceAfter.binaries[0].sha256 = 'mutated'; }]) {
    const next = fixture(true); change(next); assert.throws(() => compareBulk(fixture(), next));
  }
});
