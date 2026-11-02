import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { percentile } from './client.mjs';

export function readReport(path) {
  const report = JSON.parse(readFileSync(path));
  for (const sample of report.phases.contractAndMatrix.result.matrix) {
    if (sample.fairness) continue;
    const name = `postgres-${sample.nodeCount}-${sample.writers}-successes.jsonl`;
    const successes = readFileSync(resolve(dirname(path), name), 'utf8').trim().split('\n').map(JSON.parse);
    assert.equal(successes.length, sample.successfulCount);
    sample.maxRetries = Math.max(...successes.map(value => value.retries));
    sample.fairness = Array.from({ length: sample.writers }, (_, writer) => {
      const values = successes.filter(value => value.writer === writer);
      assert.equal(values.length, sample.iterations);
      return { writer, successfulCount: values.length, totalRetries: values.reduce((sum, value) => sum + value.retries, 0),
        maxRetries: Math.max(...values.map(value => value.retries)), totalOperationMs: values.reduce((sum, value) => sum + value.ms, 0),
        retryToSuccessMs: { p95: percentile(values.map(value => value.ms), .95), p99: percentile(values.map(value => value.ms), .99) } };
    });
  }
  return report;
}

export function comparison(baseline, optimized) {
  assert.ok(Date.parse(baseline.finishedAt) <= Date.parse(optimized.startedAt), 'Baseline and optimized runs must be sequential');
  for (const report of [baseline, optimized]) {
    assert.equal(report.status, 'pass');
    assert.equal(report.phases.contractAndMatrix.status, 'pass');
    assert.equal(report.provenance.sourceFingerprint, report.provenanceAfter.sourceFingerprint, 'Sources must stay frozen');
    assert.match(report.cleanup, /removed/);
  }
  for (const field of ['nodeCounts', 'writers', 'iterations']) assert.equal(baseline.configuration[field], optimized.configuration[field], field);
  for (const field of ['image', 'imageId', 'cpu', 'memoryBytes', 'postgresVersion', 'node']) assert.deepEqual(baseline.topology[field], optimized.topology[field], field);
  assert.deepEqual(baseline.topology.gatewayHost, optimized.topology.gatewayHost, 'Same host resource scope');
  assert.equal(baseline.provenance.rustc, optimized.provenance.rustc);
  assert.equal(baseline.provenance.cargo, optimized.provenance.cargo);
  const original = baseline.phases.contractAndMatrix.result.matrix;
  const updated = optimized.phases.contractAndMatrix.result.matrix;
  const rows = [];
  for (const base of original) {
    assert.equal(base.successfulCount, base.writers * base.iterations);
    assert.ok(base.readCount > 0 && base.observedCommits > 0);
    for (const clientMode of ['legacy', 'optimized']) {
      const sample = updated.find(value => value.clientMode === clientMode && value.nodeCount === base.nodeCount && value.writers === base.writers);
      assert.ok(sample, `Missing ${clientMode}/${base.nodeCount}/${base.writers}`);
      assert.equal(sample.successfulCount, base.successfulCount);
      assert.equal(sample.iterations, base.iterations);
      assert.ok(sample.readCount > 0 && sample.observedCommits > 0);
    }
    const summarize = sample => ({ successCount: sample.successfulCount, requestAttempts: sample.requestAttempts,
      conflicts: sample.conflictCount, snapshotReads: sample.readCount, postAttemptMs: sample.attemptMs,
      retryToSuccessMs: sample.retryToSuccessMs, throughput: sample.throughput, maxRetries: sample.maxRetries ?? null,
      fairness: sample.fairness ?? 'not recorded by original harness; derive from preserved successes JSONL' });
    rows.push({ nodeCount: base.nodeCount, writers: base.writers, baseline: summarize(base),
      serverOnly: summarize(updated.find(value => value.clientMode === 'legacy' && value.nodeCount === base.nodeCount && value.writers === base.writers)),
      serverAndClient: summarize(updated.find(value => value.clientMode === 'optimized' && value.nodeCount === base.nodeCount && value.writers === base.writers)) });
  }
  assert.equal(rows.length, 6, 'Full default six-cell baseline required');
  return { scope: 'Sequential fixture runs; same workload/readers/SSE/resources/dev build; no product SLA',
    configurations: { baseline: baseline.configuration, optimized: optimized.configuration },
    provenance: { baseline: baseline.provenance, optimized: optimized.provenance }, rows };
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [baselinePath, optimizedPath, outputPath] = process.argv.slice(2);
  assert.ok(baselinePath && optimizedPath && outputPath, 'Usage: node compare.mjs baseline/report.json optimized/report.json new-comparison.json');
  const result = comparison(readReport(baselinePath), readReport(optimizedPath));
  writeFileSync(outputPath, JSON.stringify(result, null, 2), { flag: 'wx' });
  console.log(JSON.stringify({ outputPath, cells: result.rows.length, modes: 3 }));
}
