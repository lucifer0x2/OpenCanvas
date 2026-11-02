import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { quantiles } from './single-writer-load.mjs';

const phases = ['apply', 'begin', 'commit', 'hash', 'lock', 'nodeDecode', 'nodeQuery', 'persist', 'pool', 'replay'];
const core = ['cloneUs', 'executeUs', 'sortUs', 'validateUs'];
const sum = values => values.reduce((a, b) => a + b, 0);

export function readProfiles(path) {
  return readFileSync(path, 'utf8').split('\n').filter(line => line.startsWith('{')).map(line => JSON.parse(line))
    .filter(value => value.batchProfile === true).map(value => {
      assert.deepEqual(Object.keys(value).sort(), ['batchProfile', 'coreUs', 'nodeCount', 'operationCount',
        'outcome', 'phasesUs', 'profileSequence', 'totalUs'].sort(), 'Only fixed, payload-free profile fields may be saved');
      return value;
    });
}

export function measuredProfiles(records, { nodeCount, seedCommits, warmup, samples }) {
  assert.equal(records.length, seedCommits + warmup + samples, 'Every seed/warmup/measured commit must be profiled once');
  assert.ok(records.every(value => value.outcome === 'committed'));
  assert.ok(records.every((value, index) => index === 0 || value.profileSequence === records[index - 1].profileSequence + 1),
    'Profile sequence must identify every commit exactly once in gateway order');
  const measured = records.slice(seedCommits + warmup);
  for (const value of measured) {
    assert.equal(value.nodeCount, nodeCount); assert.equal(value.operationCount, 3);
    assert.deepEqual(Object.keys(value.phasesUs).sort(), phases);
    assert.deepEqual(Object.keys(value.coreUs).sort(), core);
    const durations = [value.totalUs, ...Object.values(value.phasesUs), ...Object.values(value.coreUs)];
    assert.ok(durations.every(duration => Number.isSafeInteger(duration) && duration >= 0));
    assert.ok(sum(Object.values(value.phasesUs)) <= value.totalUs, 'Top-level phases are disjoint elapsed intervals');
    assert.ok(sum(Object.values(value.coreUs)) <= value.phasesUs.apply, 'Core subphases must fit inside apply');
  }
  assert.equal(measured.length, samples);
  const total = sum(measured.map(value => value.totalUs));
  const summarize = field => Object.fromEntries(Object.keys(measured[0][field]).sort().map(name => [name, {
    ...quantiles(measured.map(value => value[field][name] / 1000)),
    fractionOfTotalElapsed: sum(measured.map(value => value[field][name])) / total,
  }]));
  return { records: measured, summary: { serverTotalMs: quantiles(measured.map(value => value.totalUs / 1000)),
    phasesMs: summarize('phasesUs'), coreMs: summarize('coreUs'),
    interpretation: 'Elapsed wall time, including waits/scheduling. Core subphases overlap apply; fractions use sums, not summed medians. HTTP/revision GET/client/reader/SSE costs are outside serverTotal.' } };
}
