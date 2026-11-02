import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { batch, create, request } from './client.mjs';
import { Gateways } from '../infra/gateways.mjs';
import { root, sql } from '../infra/runtime.mjs';

export const indexNames = ['nodes_parent', 'render_jobs_artifact'];
export const indexSql = () => readFileSync(resolve(root, 'postgres/indexes.sql'), 'utf8');
export const schemaSql = () => ['schema.sql', 'durable-schema.sql']
  .map(file => readFileSync(resolve(root, 'postgres', file), 'utf8')).join('\n');
// Generated IDs/values are local constants, never external SQL input.
export const fixtureSql = `
INSERT INTO projects SELECT 'p'||p, 1000 FROM generate_series(1,10000) p;
INSERT INTO nodes(project_id,node_id,parent_id,kind,sort_key,props)
SELECT 'p'||p, 'n'||n, CASE WHEN n=0 THEN NULL WHEN n<50 THEN 'n0'
 ELSE 'n'||(1+(n-50)%49) END, CASE WHEN n=0 THEN 'page' ELSE 'group' END,
 n, jsonb_build_object('name',repeat('x',64))
FROM generate_series(1,20) p CROSS JOIN generate_series(0,4999) n;
INSERT INTO commits SELECT 'p'||p,r,'[]'::jsonb
FROM generate_series(1,20) p CROSS JOIN generate_series(1,1000) r;
INSERT INTO idempotency SELECT 'p'||p,'key'||r,'fixture-hash','{}'::jsonb
FROM generate_series(1,20) p CROSS JOIN generate_series(1,1000) r;
INSERT INTO shared_blobs VALUES ('artifact-fixture',decode('00','hex'));
INSERT INTO render_jobs(job_id,project_id,revision,pipeline_fingerprint,input,status,artifact_id)
SELECT 'job'||j,'p1',1000,'fixture','{}'::jsonb,
 CASE WHEN j=10000 THEN 'completed' ELSE 'pending' END,
 CASE WHEN j=10000 THEN 'artifact-fixture' ELSE NULL END
FROM generate_series(1,10000) j;
ANALYZE projects; ANALYZE nodes; ANALYZE commits; ANALYZE idempotency; ANALYZE render_jobs;`;

const nodeFields = 'node_id,parent_id,kind,sort_key,props';
export const probes = [
  { name: 'revision', query: 'SELECT revision FROM projects WHERE project_id=$1', params: ['p11'], expectedRows: 1, index: 'projects_pkey' },
  { name: 'lockedRevision', query: 'SELECT revision FROM projects WHERE project_id=$1 FOR UPDATE', params: ['p11'], expectedRows: 1, index: 'projects_pkey' },
  { name: 'nodePoint', query: `SELECT ${nodeFields} FROM nodes WHERE project_id=$1 AND node_id=$2`, params: ['p11', 'n4999'], expectedRows: 1, index: 'nodes_pkey' },
  { name: 'snapshot', query: `SELECT ${nodeFields} FROM nodes WHERE project_id=$1 ORDER BY node_id`, params: ['p11'], expectedRows: 5000 },
  { name: 'fullTableControl', query: `SELECT ${nodeFields} FROM nodes`, params: [], expectedRows: 100000 },
  { name: 'parentFk', query: 'SELECT 1 FROM ONLY nodes WHERE project_id=$1 AND parent_id=$2 FOR KEY SHARE', params: ['p11', 'n49'], expectedRows: 101, index: 'nodes_parent', added: true },
  { name: 'parentFkLeaf', query: 'SELECT 1 FROM ONLY nodes WHERE project_id=$1 AND parent_id=$2 FOR KEY SHARE', params: ['p11', 'n4999'], expectedRows: 0, index: 'nodes_parent', added: true },
  { name: 'commitsCursor', query: 'SELECT revision,changed_ids FROM commits WHERE project_id=$1 AND revision>$2 ORDER BY revision LIMIT 100', params: ['p11', 990], expectedRows: 10, index: 'commits_pkey' },
  { name: 'idempotency', query: 'SELECT request_hash,response FROM idempotency WHERE project_id=$1 AND request_key=$2', params: ['p11', 'key999'], expectedRows: 1, index: 'idempotency_pkey' },
  { name: 'artifact', query: 'SELECT 1 FROM render_jobs WHERE artifact_id=$1 LIMIT 1', params: ['artifact-fixture'], expectedRows: 1, index: 'render_jobs_artifact', added: true },
];
export function bind(probe) {
  return probe.query.replace(/\$(\d+)/g, (_, n) => {
    const value = probe.params[Number(n) - 1];
    return typeof value === 'number' ? String(value) : `'${value.replaceAll("'", "''")}'`;
  });
}

export async function mutationChecks(runtime, database) {
  const fixtureRuntime = { ...runtime, env: { ...runtime.env,
    PG_URL: runtime.env.PG_URL.replace(/\/spike$/, `/${database}`) } };
  const gateway = new Gateways(fixtureRuntime, resolve(root, 'postgres/target/debug/oc-spike-postgres'));
  const projectId = 'index-mutation';
  const send = async (revision, operations, status = 200) => {
    const result = await request(gateway.urls[0], '/batch', batch(projectId, revision, operations));
    assert.equal(result.status, status, JSON.stringify(result.body));
    return result.body;
  };
  try {
    await gateway.start(0);
    assert.equal((await request(gateway.urls[0], '/projects', { projectId })).status, 200);
    await send(0, [create('page'), create('a', 'page', 'group'), create('b', 'page', 'group'), create('child', 'a', 'group'), create('leaf', 'child', 'group')]);
    await send(1, [{ op: 'reparent_node', id: 'child', parentId: 'b', sortKey: 9 }]);
    assert.equal(sql(runtime, `SELECT parent_id FROM nodes WHERE project_id='${projectId}' AND node_id='child'`, database).trim(), 'b');
    await send(2, [{ op: 'delete_subtree', id: 'a' }]);
    assert.equal(Number(sql(runtime, `SELECT count(*) FROM nodes WHERE project_id='${projectId}'`, database)), 4);
    await send(3, [{ op: 'reparent_node', id: 'child', parentId: 'missing' }], 400);
    await send(3, [{ op: 'delete_subtree', id: 'b' }]);
    const snapshot = (await request(gateway.urls[0], `/projects/${projectId}`)).body;
    assert.equal(snapshot.revision, 4);
    assert.deepEqual(snapshot.nodes.map(node => node.id), ['page']);
    // Direct SQL must still reject a dangling parent at deferred constraint time.
    assert.throws(() => sql(runtime, `BEGIN; UPDATE nodes SET parent_id='missing' WHERE project_id='${projectId}'; COMMIT;`, database),
      error => /violates foreign key constraint/.test(String(error.stderr || error.message)));
    assert.equal(sql(runtime, `SELECT coalesce(parent_id,'NULL') FROM nodes WHERE project_id='${projectId}'`, database).trim(), 'NULL');
    return { reparent: 'pass', deleteSubtree: 'pass', failedReparentAtomic: 'pass', deferredFkRejectsDanglingParent: 'pass', revision: 4 };
  } finally { await gateway.close(); }
}
