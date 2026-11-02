import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { quantiles } from './single-writer-load.mjs';

const fields = ['batchProfile', 'coreUs', 'nodeCount', 'operationCount', 'outcome', 'phasesUs', 'profileSequence', 'totalUs'];
export function task17Profiles(path) {
  return readFileSync(path, 'utf8').split('\n').filter(line => line.startsWith('{')).map(line => JSON.parse(line))
    .filter(value => value.batchProfile === true).map(value => {
      assert.deepEqual(Object.keys(value).sort(), [...fields, ...(value.writeEvidence ? ['writeEvidence'] : [])].sort());
      if (value.writeEvidence) {
        assert.deepEqual(Object.keys(value.writeEvidence).sort(), ['fallbackReason', 'path', 'queriedNodeCount', 'validatedNodeCount']);
        assert.ok(['delta', 'full'].includes(value.writeEvidence.path));
        assert.ok([null, 'structuralOrMalformed', 'missingCertificate', 'validatorVersion', 'certificateRevision'].includes(value.writeEvidence.fallbackReason));
      }
      return value;
    });
}
export function task17Measured(records, cell, optimized) {
  assert.equal(records.length, cell.seedCommits + cell.warmup + cell.samples);
  assert.ok(records.every(value => value.outcome === 'committed'));
  const measured = records.slice(cell.seedCommits + cell.warmup);
  const durations = [], queried = [], validated = [];
  for (const [index, value] of records.entries()) {
    assert.ok(index === 0 || value.profileSequence === records[index - 1].profileSequence + 1);
    assert.equal(value.operationCount, index < cell.seedCommits ? Math.min(500, cell.nodeCount - index * 500) : 3);
  }
  for (const value of measured) {
    const counts = value.writeEvidence || { path: 'full', queriedNodeCount: value.nodeCount, validatedNodeCount: value.nodeCount };
    assert.equal(counts.path, optimized ? 'delta' : 'full');
    assert.equal(counts.queriedNodeCount, optimized ? 3 : cell.nodeCount);
    assert.equal(counts.validatedNodeCount, optimized ? 3 : cell.nodeCount);
    if (optimized) assert.equal(value.coreUs.sortUs, 0, 'Selected delta rows need no full-state sort');
    const numbers = [value.totalUs, ...Object.values(value.phasesUs), ...Object.values(value.coreUs)];
    assert.ok(numbers.every(n => Number.isSafeInteger(n) && n >= 0));
    assert.ok(Object.values(value.phasesUs).reduce((a, b) => a + b, 0) <= value.totalUs);
    durations.push(value.totalUs / 1000); queried.push(counts.queriedNodeCount); validated.push(counts.validatedNodeCount);
  }
  const summary = { serverTotalMs: quantiles(durations), queriedNodeCount: quantiles(queried), validatedNodeCount: quantiles(validated),
    writePath: optimized ? 'delta' : 'full', phaseMs: Object.fromEntries(Object.keys(measured[0].phasesUs).map(name =>
      [name, quantiles(measured.map(record => record.phasesUs[name] / 1000))])) };
  return { records: measured, summary };
}
