import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { pathToFileURL } from 'node:url';

export function compareBulk(baseline, optimized) {
  assert.ok(Date.parse(baseline.finishedAt) <= Date.parse(optimized.startedAt), 'Runs must be sequential');
  for (const report of [baseline, optimized]) {
    assert.equal(report.status, 'pass');
    assert.equal(report.phases.contractAndMatrix.status, 'pass');
    assert.equal(report.provenance.sourceFingerprint, report.provenanceAfter.sourceFingerprint);
    assert.deepEqual(report.provenance.binaries, report.provenanceAfter.binaries, 'Binaries must stay frozen');
    assert.match(report.cleanup, /removed/);
    assert.equal(report.configuration.clientModes, 'optimized', 'Compare identical optimized clients only');
    assert.equal(report.phases.contractAndMatrix.result.matrix.length, 6);
  }
  for (const field of ['nodeCounts', 'writers', 'iterations', 'clientModes', 'buildProfile']) assert.equal(baseline.configuration[field], optimized.configuration[field], field);
  for (const field of ['image', 'imageId', 'cpu', 'memoryBytes', 'postgresVersion', 'node', 'gatewayHost']) assert.deepEqual(baseline.topology[field], optimized.topology[field], field);
  assert.notEqual(baseline.topology.containerId, optimized.topology.containerId, 'Each run needs a fresh database');
  for (const field of ['rustc', 'cargo']) assert.equal(baseline.provenance[field], optimized.provenance[field], field);
  const summarize = sample => ({ successfulCount: sample.successfulCount, observedCommits: sample.observedCommits,
    postAttemptMs: sample.attemptMs, retryToSuccessMs: sample.retryToSuccessMs, requestAttempts: sample.requestAttempts,
    conflictCount: sample.conflictCount, maxRetries: sample.maxRetries, throughput: sample.throughput,
    snapshotReads: sample.readCount, fairness: sample.fairness });
  const prior = baseline.phases.contractAndMatrix.result.matrix;
  const next = optimized.phases.contractAndMatrix.result.matrix;
  const keys = new Set();
  const rows = prior.map(base => {
    const key = `${base.nodeCount}/${base.writers}`;
    assert.ok(!keys.has(key), 'Duplicate matrix cell'); keys.add(key);
    const matching = next.filter(sample => sample.nodeCount === base.nodeCount && sample.writers === base.writers);
    assert.equal(matching.length, 1, key);
    const sample = matching[0];
    for (const value of [base, sample]) {
      assert.equal(value.clientMode, 'optimized'); assert.equal(value.iterations, 12);
      assert.equal(value.successfulCount, value.writers * value.iterations);
      assert.equal(value.observedCommits, value.successfulCount);
      assert.ok(value.readCount > 0); assert.equal(value.mixedSnapshots, 0);
      assert.equal(value.fairness.length, value.writers);
      assert.ok(value.fairness.every(writer => writer.successfulCount === value.iterations));
    }
    return { nodeCount: base.nodeCount, writers: base.writers, baseline: summarize(base), optimized: summarize(sample) };
  });
  assert.deepEqual([...keys].sort(), ['100/1', '100/4', '100/8', '5000/1', '5000/4', '5000/8'].sort());
  return { scope: 'Same optimized client fixture, fresh isolated DB per side, sequential dev builds; reader+SSE remain active. No latency SLA or causal phase attribution.',
    configurations: { baseline: baseline.configuration, optimized: optimized.configuration },
    provenance: { baseline: baseline.provenance, optimized: optimized.provenance }, rows };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [before, after, destination] = process.argv.slice(2);
  assert.ok(before && after && destination, 'Usage: node compare-bulk.mjs before/report.json after/report.json new-comparison.json');
  const result = compareBulk(JSON.parse(readFileSync(before)), JSON.parse(readFileSync(after)));
  writeFileSync(destination, JSON.stringify(result, null, 2), { flag: 'wx' });
  console.log(JSON.stringify({ destination, cells: result.rows.length }));
}
