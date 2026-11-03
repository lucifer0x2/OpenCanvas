import assert from 'node:assert/strict';
import { mkdtempSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { quantiles } from './single-writer-load.mjs';
import { measuredProfiles, readProfiles } from './single-writer-profiles.mjs';
import { summarizeRepetitions } from './single-writer-summary.mjs';

const cell = { nodeCount: 5000, seedCommits: 10, warmup: 20, samples: 100 };
const record = profileSequence => ({ batchProfile: true, profileSequence, operationCount: 3,
  nodeCount: 5000, outcome: 'committed', totalUs: 1000,
  phasesUs: { hash: 5, pool: 5, begin: 10, lock: 10, replay: 10, nodeQuery: 200,
    nodeDecode: 100, apply: 200, persist: 100, commit: 100 },
  coreUs: { cloneUs: 30, executeUs: 40, validateUs: 50, sortUs: 30 } });
const records = () => Array.from({ length: 130 }, (_, n) => record(n + 1));

test('quantiles use nearest rank with the real sample count', () => {
  const result = quantiles(Array.from({ length: 100 }, (_, n) => n + 1));
  assert.deepEqual(result, { count: 100, p50: 50, p95: 95, p99: 99, min: 1, max: 100 });
  assert.throws(() => quantiles([])); assert.throws(() => quantiles([NaN]));
});

test('profiles exclude seed and warmup and preserve nested elapsed phase interpretation', () => {
  const result = measuredProfiles(records(), cell);
  assert.equal(result.records[0].profileSequence, 31);
  assert.equal(result.records.length, 100);
  assert.equal(result.summary.serverTotalMs.p50, 1);
  assert.equal(result.summary.phasesMs.apply.fractionOfTotalElapsed, .2);
  assert.equal(result.summary.coreMs.validateUs.fractionOfTotalElapsed, .05);
});

test('profile evidence rejects missing commits, error outcomes, wrong workload and invalid phase intervals', () => {
  for (const mutate of [values => values.pop(), values => { values[0].outcome = 'error'; },
    values => { values[30].nodeCount = 100; }, values => { values[30].operationCount = 1; },
    values => { values[30].profileSequence = 1; },
    values => { values[30].totalUs = 1; }, values => { values[30].coreUs.cloneUs = 1000; },
    values => { delete values[30].phasesUs.lock; }]) {
    const values = records(); mutate(values); assert.throws(() => measuredProfiles(values, cell));
  }
});

test('profile parser ignores readiness but rejects unapproved payload fields', () => {
  const directory = mkdtempSync(join(tmpdir(), 'single-writer-profiles-'));
  try {
    const path = join(directory, 'gateway.log');
    writeFileSync(path, JSON.stringify({ ready: true, port: 1 }) + '\n' + JSON.stringify(record(1)) + '\n');
    assert.deepEqual(readProfiles(path), [record(1)]);
    writeFileSync(path, JSON.stringify({ ...record(1), projectId: 'must-not-be-copied' }) + '\n');
    assert.throws(() => readProfiles(path));
  } finally { rmSync(directory, { recursive: true }); }
});

function repetition(second = false) {
  const provenance = { binary: { sha256: 'same-binary' }, source: { fingerprint: 'same-source' } };
  const report = { status: 'pass', startedAt: second ? '2026-11-02T01:00:00Z' : '2026-11-02T00:00:00Z',
    finishedAt: second ? '2026-11-02T01:01:00Z' : '2026-11-02T00:01:00Z',
    configuration: { writers: 1, samples: 100, warmup: 20 }, provenance,
    provenanceAfter: structuredClone(provenance), cleanup: { containerAbsent: true },
    topology: { containerId: second ? 'fresh2' : 'fresh1', cpu: 2 }, cells: [] };
  report.cells = [100, 5000].map(nodeCount => ({ nodeCount, writers: 1, samples: 100, warmup: 20,
    observedEvents: 120, observedCommits: 120, successfulCount: 120, mixedSnapshots: 0,
    replayVerified: true, measuredReadCount: 1, provenanceBefore: provenance,
    provenanceAfter: provenance, profile: { phasesMs: Object.fromEntries(['nodeQuery', 'nodeDecode', 'apply', 'persist', 'commit']
      .map(name => [name, { fractionOfTotalElapsed: .1 }])) } }));
  return report;
}

test('repetition comparison requires identical frozen input, workload, resources and completed traffic', () => {
  assert.equal(summarizeRepetitions([repetition(), repetition(true)]).rows.length, 4);
  for (const mutate of [value => { value.topology.containerId = 'fresh1'; },
    value => { value.configuration.samples = 12; }, value => { value.topology.cpu = 1; },
    value => { value.provenanceAfter.binary.sha256 = 'mutated'; },
    value => { value.cells[0].observedEvents = 0; }, value => { value.cells[0].measuredReadCount = 0; },
    value => { value.cleanup.containerAbsent = false; }]) {
    const next = repetition(true); mutate(next);
    assert.throws(() => summarizeRepetitions([repetition(), next]));
  }
});
