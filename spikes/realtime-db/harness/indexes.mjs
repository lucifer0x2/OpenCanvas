import assert from 'node:assert/strict';
import { randomBytes } from 'node:crypto';
import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { sql } from '../infra/runtime.mjs';
import { bind, fixtureSql, indexNames, indexSql, mutationChecks, probes, schemaSql } from './indexes-fixture.mjs';

export function planNodes(plan) {
  return [plan, ...(plan.Plans || []).flatMap(planNodes)];
}
export function checkPlan(probe, raw, stage) {
  assert.ok(Array.isArray(raw) && raw.length === 1 && raw[0].Plan, `${probe.name}: missing actual plan`);
  const nodes = planNodes(raw[0].Plan);
  assert.equal(raw[0].Plan['Actual Rows'], probe.expectedRows, `${probe.name}: wrong actual cardinality`);
  assert.ok(Number.isFinite(raw[0]['Execution Time']), `${probe.name}: missing execution timing`);
  assert.ok(nodes.some(node => Number.isFinite(node['Shared Hit Blocks'])), `${probe.name}: missing buffers`);
  if (probe.index && (!probe.added || stage === 'after')) {
    assert.ok(nodes.some(node => node['Index Name'] === probe.index), `${stage}/${probe.name}: expected natural ${probe.index} scan`);
  }
  if (probe.added && stage === 'before') {
    assert.ok(nodes.every(node => !indexNames.includes(node['Index Name'])), 'before plan contains new index');
  }
  return { actualRows: raw[0].Plan['Actual Rows'], executionMs: raw[0]['Execution Time'],
    sharedHitBlocks: raw[0].Plan['Shared Hit Blocks'], sharedReadBlocks: raw[0].Plan['Shared Read Blocks'],
    scans: nodes.filter(node => /Scan/.test(node['Node Type'])).map(node => ({
      type: node['Node Type'], index: node['Index Name'], actualRows: node['Actual Rows'],
      loops: node['Actual Loops'], rowsRemovedByFilter: node['Rows Removed by Filter'] || 0,
      sharedHitBlocks: node['Shared Hit Blocks'], sharedReadBlocks: node['Shared Read Blocks'],
      indexCond: node['Index Cond'], filter: node.Filter,
    })) };
}
export function checkDefinitions(definitions) {
  assert.ok(Array.isArray(definitions) && definitions.length >= 6, 'Index inventory must not be empty');
  for (const name of indexNames) assert.equal(definitions.filter(row => row.name === name).length, 1, `${name} missing/duplicated`);
  const signatures = definitions.map(row => row.table + row.definition.replace(/^CREATE (UNIQUE )?INDEX \S+ ON \S+ /, ''));
  assert.equal(new Set(signatures).size, signatures.length, 'Duplicate index definitions');
  assert.match(definitions.find(row => row.name === 'nodes_parent').definition, /btree \(project_id, parent_id\)$/);
  assert.match(definitions.find(row => row.name === 'render_jobs_artifact').definition, /btree \(artifact_id\) WHERE \(artifact_id IS NOT NULL\)$/);
}

export async function indexEvidence(runtime, outputDir) {
  // A separate owned DB also isolates ANALYZE statistics and fixture cardinality.
  const database = `index_plan_${randomBytes(5).toString('hex')}`;
  const execute = query => sql(runtime, query, database);
  const save = (name, data) => writeFileSync(resolve(outputDir, name), JSON.stringify(data, null, 2));
  const evidence = { databaseScope: 'fresh disposable database in this run-owned container',
    fixture: { projects: 10000, nodes: 100000, populatedProjects: 20, nodesPerPopulatedProject: 5000,
      commits: 20000, idempotency: 20000, jobs: 10000, completedArtifacts: 1 },
    methodology: 'Literal-bound equivalents of gateway parameterized queries; unforced planner; ANALYZE before both stages; one unrecorded warmup and one recorded observation per probe/stage; no timing threshold',
    probes, before: {}, after: {}, files: ['index-fixture.sql', 'index-definitions.sql', 'index-evidence.json'],
    caveats: ['Full snapshots still materialize all 5000 project nodes/JSONB; this index does not remove that cost.',
      'Parent probes mirror FK predicates; EXPLAIN does not expose the internal deferred-trigger SELECT plan.',
      'Seq Scan is valid for broad/full-table reads; timing observations are not comparative workload percentiles.'] };
  sql(runtime, `CREATE DATABASE ${database}`, 'postgres');
  try {
    const schema = schemaSql();
    execute(schema);
    execute(fixtureSql);
    writeFileSync(resolve(outputDir, 'index-fixture.sql'), `${schema}\n${fixtureSql}`);
    writeFileSync(resolve(outputDir, 'index-definitions.sql'), indexSql());
    evidence.postgresVersion = execute('SELECT version()').trim();
    evidence.observedCardinalities = JSON.parse(execute(`SELECT json_build_object(
      'projects',(SELECT count(*) FROM projects),'nodes',(SELECT count(*) FROM nodes),
      'commits',(SELECT count(*) FROM commits),'idempotency',(SELECT count(*) FROM idempotency),
      'jobs',(SELECT count(*) FROM render_jobs),'completedArtifacts',(SELECT count(*) FROM render_jobs WHERE artifact_id IS NOT NULL))`));
    for (const [table, count] of Object.entries(evidence.observedCardinalities)) assert.equal(count, evidence.fixture[table]);
    evidence.beforeIndexes = JSON.parse(execute(`SELECT json_agg(indexname ORDER BY indexname) FROM pg_indexes WHERE schemaname='public'`));
    assert.ok(indexNames.every(name => !evidence.beforeIndexes.includes(name)), 'Before fixture must not have additive indexes');
    evidence.settings = JSON.parse(execute(`SELECT json_object_agg(name,setting) FROM pg_settings WHERE name IN
      ('enable_seqscan','enable_indexscan','enable_bitmapscan','random_page_cost','seq_page_cost','effective_cache_size','shared_buffers','work_mem','default_statistics_target')`));
    assert.equal(evidence.settings.enable_seqscan, 'on');
    assert.equal(evidence.settings.enable_indexscan, 'on');
    assert.equal(evidence.settings.enable_bitmapscan, 'on');
    for (const stage of ['before', 'after']) {
      if (stage === 'after') execute(`${indexSql()}\nANALYZE nodes; ANALYZE render_jobs;`);
      for (const probe of probes) {
        const explain = `EXPLAIN (ANALYZE, BUFFERS, FORMAT JSON) ${bind(probe)}`;
        execute(explain); // Equal per-stage warmup; do not claim a cold-cache comparison.
        const raw = JSON.parse(execute(explain));
        const filename = `index-${stage}-${probe.name}.json`;
        save(filename, { query: probe.query, params: probe.params, boundQuery: bind(probe), explain: raw });
        evidence.files.push(filename);
        evidence[stage][probe.name] = checkPlan(probe, raw, stage);
      }
    }
    evidence.indexes = JSON.parse(execute(`SELECT json_agg(json_build_object('name',indexname,
      'table',tablename,'definition',indexdef,'bytes',pg_relation_size((schemaname||'.'||indexname)::regclass)) ORDER BY indexname)
      FROM pg_indexes WHERE schemaname='public'`));
    checkDefinitions(evidence.indexes);
    evidence.mutations = await mutationChecks(runtime, database);
    evidence.status = 'pass';
    return { status: evidence.status, evidence: 'index-evidence.json', fixture: evidence.fixture,
      indexes: evidence.indexes.filter(row => indexNames.includes(row.name)), mutations: evidence.mutations };
  } catch (error) {
    evidence.status = 'fail'; evidence.error = error.message;
    throw error;
  } finally {
    // No other runtime/database/index is modified or removed.
    try { sql(runtime, `DROP DATABASE ${database}`, 'postgres'); evidence.cleanup = 'disposable index fixture database removed'; }
    finally { save('index-evidence.json', evidence); }
  }
}
