import assert from 'node:assert/strict';
import test from 'node:test';
import { task17Measured } from './task17-profiles.mjs';

const cell = { nodeCount: 100, seedCommits: 1, warmup: 20, samples: 100 };
function records(optimized) {
  return Array.from({ length: 121 }, (_, index) => ({ profileSequence: index + 1, outcome: 'committed',
    operationCount: index ? 3 : 100, nodeCount: optimized ? 3 : 100, totalUs: 1000,
    phasesUs: { apply: 200, commit: 300 }, coreUs: { cloneUs: 10, executeUs: 10, sortUs: 0, validateUs: 10 },
    ...(optimized ? { writeEvidence: { path: 'delta', fallbackReason: null, queriedNodeCount: 3, validatedNodeCount: 3 } } : {}) }));
}
test('Task17 profiling verifies selected row counts and warmup mapping', () => {
  const full = task17Measured(records(false), cell, false), delta = task17Measured(records(true), cell, true);
  assert.equal(full.summary.queriedNodeCount.p50, 100); assert.equal(delta.summary.queriedNodeCount.p50, 3);
  assert.equal(delta.records[0].profileSequence, 22); assert.equal(delta.records.length, 100);
});
test('Task17 profiling rejects full-state regression, missing commits and sorted full candidates', () => {
  const wrong = records(true); wrong[30].writeEvidence.queriedNodeCount = 100;
  assert.throws(() => task17Measured(wrong, cell, true));
  assert.throws(() => task17Measured(records(false).slice(1), cell, false));
  const sorted = records(true); sorted[30].coreUs.sortUs = 5;
  assert.throws(() => task17Measured(sorted, cell, true));
});
