import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';

const [beforeCounts, afterCounts, beforeScaling, afterScaling, destination] = process.argv.slice(2);
assert.ok(destination, 'Usage: node compare-bulk-diagnostics.mjs before-counts.json after-counts.json before-scaling.json after-scaling.json new-output.json');
const read = path => JSON.parse(readFileSync(path));
function pair(before, after) {
  for (const report of [before, after]) {
    assert.equal(report.status, 'pass'); assert.match(report.cleanup, /removed/);
    assert.equal(report.provenance.sourceFingerprint, report.provenanceAfter.sourceFingerprint);
    assert.deepEqual(report.provenance.binaries, report.provenanceAfter.binaries);
  }
  assert.ok(Date.parse(before.finishedAt) <= Date.parse(after.startedAt), 'Runs must be sequential');
  assert.notEqual(before.topology.containerId, after.topology.containerId, 'Fresh database per side');
  for (const field of ['image', 'imageId', 'cpu', 'memoryBytes', 'postgresVersion', 'node', 'gatewayHost']) assert.deepEqual(before.topology[field], after.topology[field], field);
  for (const field of ['rustc', 'cargo']) assert.equal(before.provenance[field], after.provenance[field], field);
}
const originalCounts = read(beforeCounts), optimizedCounts = read(afterCounts);
pair(originalCounts, optimizedCounts);
assert.equal(originalCounts.method, optimizedCounts.method);
assert.deepEqual(originalCounts.proofProvenance, optimizedCounts.proofProvenance);
assert.equal(originalCounts.samples.length, 7); assert.equal(optimizedCounts.samples.length, 7);
const protocol = originalCounts.samples.map(before => {
  const matching = optimizedCounts.samples.filter(after => after.kind === before.kind && after.changedRows === before.changedRows);
  assert.equal(matching.length, 1);
  for (const value of [before, matching[0]]) {
    assert.equal(value.protocol.synchronizationCycles, value.protocol.readyForQuery);
    assert.equal(value.protocol.frontend.Q, 2);
    assert.equal(value.protocol.frontend.P, value.protocol.frontend.E);
    assert.equal(value.protocol.sqlExecutions, value.protocol.frontend.E + value.protocol.frontend.Q);
  }
  return { kind: before.kind, changedRows: before.changedRows, baseline: before.protocol, optimized: matching[0].protocol };
});
const originalScaling = read(beforeScaling), optimizedScaling = read(afterScaling);
pair(originalScaling, optimizedScaling);
assert.deepEqual(originalScaling.configuration, optimizedScaling.configuration);
assert.deepEqual(originalScaling.driverProvenance, optimizedScaling.driverProvenance);
assert.deepEqual(originalScaling.clientProvenance, optimizedScaling.clientProvenance);
assert.deepEqual(originalScaling.samples.map(value => value.changedRows), [1, 3, 500]);
assert.deepEqual(optimizedScaling.samples.map(value => value.changedRows), [1, 3, 500]);
for (const [path, report] of [[beforeScaling, originalScaling], [afterScaling, optimizedScaling]]) {
  for (const summary of report.samples) {
    const raw = readFileSync(resolve(dirname(path), `bulk-${summary.changedRows}-samples.jsonl`), 'utf8').trim().split('\n').map(JSON.parse);
    assert.equal(raw.length, 12); assert.equal(summary.successfulCount, 12); assert.equal(summary.observedCommits, 12);
    for (const value of raw) assert.ok(value.wholeOperationMs >= value.revisionGetMs + value.postMs);
    for (const field of ['postMs', 'wholeOperationMs', 'revisionGetMs']) {
      const sorted = raw.map(value => value[field]).sort((a, b) => a - b);
      for (const [quantile, fraction] of [['p50', .5], ['p95', .95], ['p99', .99]]) assert.equal(summary[field][quantile], sorted[Math.ceil(raw.length * fraction) - 1]);
    }
    assert.ok(raw.every((value, index) => value.committedRevision === raw[0].committedRevision + index));
  }
}
const scaling = originalScaling.samples.map((before, index) => ({ changedRows: before.changedRows, baseline: before, optimized: optimizedScaling.samples[index] }));
const result = { method: { protocol: originalCounts.definitions, scaling: originalScaling.configuration,
  caveat: 'Sync cycles include asynchronous prepared-statement Close+Sync; they are not physical TCP RTTs or individually awaited critical-path roundtrips. SQL executions count top-level commands, so writable CTE subcommands remain one execution.' },
  evidence: { beforeCounts, afterCounts, beforeScaling, afterScaling }, protocol, scaling };
writeFileSync(destination, JSON.stringify(result, null, 2), { flag: 'wx' });
console.log(JSON.stringify({ destination, protocolCases: protocol.length, scalingCases: scaling.length }));
