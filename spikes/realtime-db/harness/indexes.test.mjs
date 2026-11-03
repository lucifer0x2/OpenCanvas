import assert from 'node:assert/strict';
import { test } from 'node:test';
import { bind, probes } from './indexes-fixture.mjs';
import { checkDefinitions, checkPlan } from './indexes.mjs';

const plan = (index = 'nodes_parent', rows = 101) => [{ 'Execution Time': 1,
  Plan: { 'Node Type': 'LockRows', 'Actual Rows': rows, 'Shared Hit Blocks': 3,
    Plans: [{ 'Node Type': 'Bitmap Heap Scan', 'Actual Rows': rows, 'Shared Hit Blocks': 3,
      Plans: [{ 'Node Type': 'Bitmap Index Scan', 'Index Name': index, 'Actual Rows': rows, 'Shared Hit Blocks': 1 }] }] } }];
const probe = probes.find(probe => probe.name === 'parentFk');
test('plan checks require observed rows, buffers, timing and correct natural index', () => {
  assert.throws(() => checkPlan(probe, [], 'after'), /missing actual plan/);
  assert.throws(() => checkPlan(probe, plan('nodes_parent', 0), 'after'), /cardinality/);
  assert.throws(() => checkPlan(probe, plan('nodes_pkey'), 'after'), /expected natural/);
  assert.throws(() => checkPlan(probe, plan(), 'before'), /before plan/);
  assert.equal(checkPlan(probe, plan(), 'after').actualRows, 101);
  const absentBuffers = [{ 'Execution Time': 1, Plan: { 'Node Type': 'Seq Scan', 'Actual Rows': 101 } }];
  assert.throws(() => checkPlan(probe, absentBuffers, 'after'), /buffers/);
});
test('empty index inventory cannot pass and SQL binding preserves predicates', () => {
  assert.throws(() => checkDefinitions([]), /must not be empty/);
  assert.equal(bind(probes.find(probe => probe.name === 'commitsCursor')),
    "SELECT revision,changed_ids FROM commits WHERE project_id='p11' AND revision>990 ORDER BY revision LIMIT 100");
  assert.equal(bind({ query: 'SELECT $1', params: ["a'b"] }), "SELECT 'a''b'");
});
