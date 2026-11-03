import assert from 'node:assert/strict';
import { readFileSync, writeFileSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { pathToFileURL } from 'node:url';
import { quantiles } from './single-writer-load.mjs';
import { task17Measured } from './task17-profiles.mjs';
import { frozenTask17 } from './task17-provenance.mjs';

export function verifyTask17Raw(report, directory) {
  const read = name => readFileSync(resolve(directory, name), 'utf8').trim().split('\n').map(line => JSON.parse(line));
  for (const cell of report.cells) {
    const attempts = read(`${cell.nodeCount}-attempts.jsonl`), successes = read(`${cell.nodeCount}-successes.jsonl`);
    assert.equal(attempts.length, 120); assert.equal(successes.length, 120);
    assert.equal(new Set(attempts.map(value => value.key)).size, 120);
    assert.ok(attempts.every(value => value.status === 200 && value.retry === 0 && value.networkRetry === 0));
    assert.deepEqual(quantiles(attempts.filter(value => value.measured).map(value => value.ms)), cell.postMs);
    assert.deepEqual(quantiles(successes.filter(value => value.measured).map(value => value.ms)), cell.wholeOperationMs);
    assert.deepEqual(quantiles(successes.filter(value => value.measured).map(value => value.revisionReadMs)), cell.revisionReadMs);
    const measured = task17Measured(read(`${cell.nodeCount}-profiles-all.jsonl`), cell, report.stage !== 'baseline');
    assert.deepEqual(measured.summary, cell.profile);
    assert.deepEqual(measured.records, read(`${cell.nodeCount}-profiles-measured.jsonl`));
    const reads = read(`${cell.nodeCount}-reads.jsonl`);
    assert.ok(reads.length === cell.readCount || reads.length === cell.readCount + 1); cell.rawReadCount = reads.length;
    assert.deepEqual(quantiles(reads.map(value => value.ms)), cell.continuousReadMs);
  }
}
export function compareTask17(reports) {
  assert.deepEqual(reports.map(report => report.stage), ['baseline', 'delta-only', 'delta-redis']);
  const [baseline, off, on] = reports;
  assert.equal(new Set(reports.map(report => report.topology.containerId)).size, 3);
  for (const report of reports) {
    assert.equal(report.status, 'pass'); frozenTask17(report.provenance, report.provenanceAfter);
    assert.deepEqual(report.configuration, { ...baseline.configuration, cache: { ...baseline.configuration.cache, enabled: report.stage === 'delta-redis' } });
    assert.deepEqual(report.durability, { fsync: 'on', synchronousCommit: 'on', fullPageWrites: 'on' });
    for (const name of ['image', 'imageId', 'cpu', 'memoryBytes', 'postgresVersion', 'gatewayHost']) assert.deepEqual(report.topology[name], baseline.topology[name]);
    for (const name of ['image', 'imageId', 'cpu', 'memoryBytes', 'maxmemoryBytes', 'eviction', 'persistence', 'redisVersion']) assert.deepEqual(report.redisTopology[name], baseline.redisTopology[name]);
    for (const name of ['driver', 'task17Driver', 'rustc', 'cargo', 'node']) assert.deepEqual(report.provenance[name], baseline.provenance[name]);
    assert.ok(Object.values(report.cleanup).every(value => value === true)); assert.ok(Object.values(report.redisCleanup).every(value => value === true));
    assert.deepEqual(report.databaseCounts, { projects: 2, commits: 251, idempotency: 251 });
    assert.deepEqual(report.cells.map(cell => cell.nodeCount), [100, 5000]);
    for (const cell of report.cells) {
      assert.equal(cell.writers, 1); assert.equal(cell.samples, 100); assert.equal(cell.warmup, 20);
      assert.equal(cell.observedEvents, 120); assert.equal(cell.observedCommits, 120);
      assert.equal(cell.mixedSnapshots, 0); assert.equal(cell.replayVerified, true); assert.ok(cell.measuredReadCount > 0);
      frozenTask17(report.provenance, cell.provenanceBefore); frozenTask17(report.provenance, cell.provenanceAfter);
    }
  }
  assert.ok(Date.parse(baseline.finishedAt) <= Date.parse(off.startedAt)); assert.ok(Date.parse(off.finishedAt) <= Date.parse(on.startedAt));
  frozenTask17(off.provenance, on.provenance);
  assert.notEqual(baseline.provenance.source.fingerprint, off.provenance.source.fingerprint);
  const rows = baseline.cells.map((base, index) => ({ nodeCount: base.nodeCount,
    baseline: { postMs: base.postMs, wholeOperationMs: base.wholeOperationMs, continuousReadMs: base.continuousReadMs, profile: base.profile },
    deltaOnly: { postMs: off.cells[index].postMs, wholeOperationMs: off.cells[index].wholeOperationMs, continuousReadMs: off.cells[index].continuousReadMs, cacheMetrics: off.cells[index].cacheMetrics, profile: off.cells[index].profile },
    deltaRedis: { postMs: on.cells[index].postMs, wholeOperationMs: on.cells[index].wholeOperationMs, continuousReadMs: on.cells[index].continuousReadMs, cacheMetrics: on.cells[index].cacheMetrics, profile: on.cells[index].profile },
    deltaWholeP50Ratio: base.wholeOperationMs.p50 / off.cells[index].wholeOperationMs.p50,
    cacheWholeP50Ratio: off.cells[index].wholeOperationMs.p50 / on.cells[index].wholeOperationMs.p50 }));
  return { status: 'pass', method: 'Sequential baseline/delta-only/delta+Redis. Same immutable client, dev build, one writer and PG/Redis resources. Each stage fresh DB. Per-cell nearest-rank quantiles; no pooled samples or SLA.', rows };
}
if (process.argv[1] && import.meta.url === pathToFileURL(resolve(process.argv[1])).href) {
  const [base, off, on, output] = process.argv.slice(2);
  assert.ok(base && off && on && output, 'Usage: task17-compare.mjs BASE/report.json OFF/report.json ON/report.json OUTPUT');
  const paths = [base, off, on], reports = paths.map(path => JSON.parse(readFileSync(path, 'utf8')));
  reports.forEach((report, index) => verifyTask17Raw(report, dirname(paths[index])));
  const result = compareTask17(reports); result.rawReports = paths;
  writeFileSync(output, JSON.stringify(result, null, 2), { flag: 'wx' });
  console.log(JSON.stringify({ status: result.status, rows: result.rows.length, rawEvidenceVerified: true, output }));
}
