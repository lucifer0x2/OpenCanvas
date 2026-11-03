import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { quantiles } from './single-writer-load.mjs';
import { measuredProfiles } from './single-writer-profiles.mjs';
import { assertFrozen } from './single-writer-provenance.mjs';

const topologyFields = ['image', 'imageId', 'cpu', 'memoryBytes', 'postgresVersion', 'node', 'gatewayHost'];
const sum = values => values.reduce((a, b) => a + b, 0);

export function summarizeRepetitions(reports) {
  assert.equal(reports.length, 2, 'Require exactly two complete baseline repetitions');
  const [first, second] = reports;
  assert.deepEqual(first.configuration, second.configuration, 'Configuration must match');
  assertFrozen(first.provenance, second.provenance);
  assert.ok(Date.parse(first.finishedAt) <= Date.parse(second.startedAt), 'Runs must be sequential');
  assert.notEqual(first.topology.containerId, second.topology.containerId, 'Each repetition needs a fresh database');
  for (const field of topologyFields) assert.deepEqual(first.topology[field], second.topology[field], `Same ${field}`);
  const rows = [];
  for (const [run, report] of reports.entries()) {
    assert.equal(report.status, 'pass'); assertFrozen(report.provenance, report.provenanceAfter);
    assert.ok(Object.values(report.cleanup).every(value => value === true));
    assert.deepEqual(report.cells.map(value => value.nodeCount), [100, 5000]);
    for (const cell of report.cells) {
      assert.equal(cell.writers, 1); assert.equal(cell.samples, report.configuration.samples);
      assert.equal(cell.warmup, report.configuration.warmup);
      assert.equal(cell.observedEvents, cell.warmup + cell.samples);
      assert.equal(cell.observedCommits, cell.warmup + cell.samples);
      assert.equal(cell.successfulCount, cell.warmup + cell.samples);
      assert.equal(cell.mixedSnapshots, 0); assert.equal(cell.replayVerified, true);
      assert.ok(cell.measuredReadCount > 0);
      assertFrozen(report.provenance, cell.provenanceBefore); assertFrozen(report.provenance, cell.provenanceAfter);
      const phase = cell.profile.phasesMs;
      rows.push({ run: run + 1, nodeCount: cell.nodeCount, samples: cell.samples,
        postMs: cell.postMs, wholeOperationMs: cell.wholeOperationMs, serverTotalMs: cell.profile.serverTotalMs,
        phasesMs: phase, coreMs: cell.profile.coreMs, measuredReadCount: cell.measuredReadCount,
        rawReadCount: cell.rawReadCount, elapsedTotalsMs: cell.elapsedTotalsMs,
        fullStateFraction: sum(['nodeQuery', 'nodeDecode', 'apply'].map(name => phase[name].fractionOfTotalElapsed)),
        persistenceFraction: phase.persist.fractionOfTotalElapsed, commitFraction: phase.commit.fractionOfTotalElapsed });
    }
  }
  return { status: 'pass', method: 'Two repetitions of the same profiled baseline; no optimized implementation or speedup claim.',
    configuration: first.configuration, provenance: first.provenance,
    quantileMethod: 'Nearest rank, per run and node count, warmup excluded. No pooling or best-of selection.', rows };
}

export function verifyRaw(report, directory) {
  const read = suffix => readFileSync(resolve(directory, suffix), 'utf8').trim().split('\n').map(line => JSON.parse(line));
  for (const cell of report.cells) {
    const attempts = read(`${cell.nodeCount}-attempts.jsonl`);
    const successes = read(`${cell.nodeCount}-successes.jsonl`);
    const profiles = read(`${cell.nodeCount}-profiles-all.jsonl`);
    assert.equal(attempts.length, cell.warmup + cell.samples);
    assert.equal(successes.length, attempts.length);
    assert.equal(new Set(attempts.map(value => value.key)).size, attempts.length);
    assert.ok(attempts.every(value => value.status === 200 && value.retry === 0 && value.networkRetry === 0));
    assert.deepEqual(quantiles(attempts.filter(value => value.measured).map(value => value.ms)), cell.postMs);
    assert.deepEqual(quantiles(successes.filter(value => value.measured).map(value => value.ms)), cell.wholeOperationMs);
    assert.deepEqual(quantiles(successes.filter(value => value.measured).map(value => value.revisionReadMs)), cell.revisionReadMs);
    const calculated = measuredProfiles(profiles, cell);
    assert.deepEqual(calculated.summary, cell.profile);
    assert.deepEqual(calculated.records, read(`${cell.nodeCount}-profiles-measured.jsonl`));
    cell.elapsedTotalsMs = { total: sum(calculated.records.map(value => value.totalUs)) / 1000,
      phases: Object.fromEntries(Object.keys(calculated.records[0].phasesUs).map(name => [name,
        sum(calculated.records.map(value => value.phasesUs[name])) / 1000])),
      core: Object.fromEntries(Object.keys(calculated.records[0].coreUs).map(name => [name,
        sum(calculated.records.map(value => value.coreUs[name])) / 1000])) };
    // The workload captures readCount before finally awaits the one in-flight
    // reader request; the raw array also retains that request's completion.
    const rawReadCount = read(`${cell.nodeCount}-reads.jsonl`).length;
    assert.ok(rawReadCount === cell.readCount || rawReadCount === cell.readCount + 1);
    cell.rawReadCount = rawReadCount;
  }
}

if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [firstPath, secondPath, output] = process.argv.slice(2);
  assert.ok(firstPath && secondPath && output, 'Usage: node single-writer-summary.mjs RUN1/report.json RUN2/report.json OUTPUT.json');
  const reports = [firstPath, secondPath].map(path => JSON.parse(readFileSync(path, 'utf8')));
  for (const [index, path] of [firstPath, secondPath].entries()) verifyRaw(reports[index], dirname(resolve(path)));
  const summary = summarizeRepetitions(reports);
  summary.rawReports = [firstPath, secondPath];
  writeFileSync(output, JSON.stringify(summary, null, 2), { flag: 'wx' });
  console.log(JSON.stringify({ status: summary.status, rows: summary.rows.length, rawEvidenceVerified: true, output }));
}
