import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { batch, create, key, request, update } from './client.mjs';
import { docker, root, sql } from '../infra/runtime.mjs';
import { Gateways } from '../infra/gateways.mjs';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
export async function migration(runtime, gateways) {
  const projectId = `migration-${key()}`;
  const [a, b] = gateways.urls;
  sql(runtime, readFileSync(resolve(root, 'infra/migration-v2.sql'), 'utf8'));
  const v2 = new Gateways(runtime, resolve(root, '.runtime/v2-target/debug/oc-spike-postgres'));
  try {
  await gateways.kill(0);
  await v2.start(0, gateways.entries[0].port);
  assert.equal((await request(a, '/projects', { projectId, description: 'client-v2' })).status, 200);
  assert.equal((await request(a, '/batch', batch(projectId, 0, [create('root')]))).status, 200);
  assert.equal((await request(a, `/projects/${projectId}`)).body.description, 'client-v2');
  assert.equal((await request(b, '/batch', batch(projectId, 1, [update('root', { name: 'v1-after-additive' })]))).status, 200);
  assert.equal(sql(runtime, `SELECT description FROM projects WHERE project_id='${projectId}'`).trim(), 'client-v2');
  await v2.kill(0);
  const restart = await gateways.restart(0, 0); // Actual rollback from feature-v2 binary to v1.
  assert.equal((await request(gateways.urls[0], `/projects/${projectId}`)).body.revision, 2);
  assert.equal((await request(a, '/batch', batch(projectId, 2, [update('root', { name: 'v1-rollback' })]))).status, 200);
  assert.equal((await request(a, `/projects/${projectId}`)).body.description, undefined);
  await v2.start(0);
  assert.equal((await request(v2.urls[0], `/projects/${projectId}`)).body.description, 'client-v2');
  const resourceBytes = [...Buffer.from('synthetic-backup-font-and-asset-v1')];
  const resource = await request(b, '/resources', { kind: 'font', bytes: resourceBytes });
  assert.equal(resource.status, 200, JSON.stringify(resource.body));
  const before = await request(b, `/projects/${projectId}`);
  const manifestQuery = `SELECT json_build_object(
    'blobs',(SELECT json_agg(json_build_array(blob_id,encode(sha256(bytes),'hex')) ORDER BY blob_id) FROM shared_blobs),
    'resources',(SELECT json_agg(json_build_array(resource_id,kind) ORDER BY resource_id,kind) FROM resources),
    'staging',(SELECT json_agg(json_build_array(upload_id,project_id) ORDER BY upload_id) FROM staged_batches),
    'jobs',(SELECT json_agg(json_build_array(job_id,status,artifact_id,input) ORDER BY job_id) FROM render_jobs))`;
  const backupManifest = JSON.parse(sql(runtime, manifestQuery));
  assert.ok(backupManifest.blobs.length > 0);
  assert.ok(backupManifest.blobs.every(([id, digest]) => id === digest));
  const dump = docker(['exec', runtime.manifest.containerId, 'pg_dump', '-U', 'postgres', '-d', 'spike', '-Fc'], { encoding: null, maxBuffer: 64 * 1024 * 1024 });
  writeFileSync(resolve(runtime.directory, 'backup.dump'), dump, { mode: 0o600 });
  sql(runtime, 'CREATE DATABASE restored', 'postgres');
  docker(['exec', '-i', runtime.manifest.containerId, 'pg_restore', '-U', 'postgres', '-d', 'restored', '--exit-on-error'], { input: dump, encoding: null, maxBuffer: 64 * 1024 * 1024 });
  assert.deepEqual(JSON.parse(sql(runtime, manifestQuery, 'restored')), backupManifest);
  const originalEnv = runtime.env;
  const oldEntries = gateways.entries;
  const restored = new Gateways({ ...runtime, env: { ...originalEnv, PG_URL: originalEnv.PG_URL.replace(/\/spike$/, '/restored') } }, gateways.executable);
  try {
    await restored.start(0);
    assert.deepEqual((await request(restored.urls[0], `/projects/${projectId}`)).body, before.body);
    const restoredResource = await request(restored.urls[0], `/resources/${resource.body.resourceId}`);
    assert.equal(restoredResource.status, 200);
    assert.deepEqual(restoredResource.body.bytes, resourceBytes);
    assert.equal(restoredResource.body.sha256, hash(Buffer.from(resourceBytes)));
    return { projectId, additiveMigration: 'pass', clientV1AfterV2: 'pass', clientV2Description: 'client-v2',
      codeRollback: 'pass: actual compat-v2 feature binary replaced by v1 binary; v1 writes succeed and later v2 reads retain description',
      gatewayRestart: restart, backupRestore: 'pass', dumpBytes: dump.length, dumpSha256: hash(dump),
      restoredRevision: before.body.revision, resourceSha256: restoredResource.body.sha256,
      restoredBlobCount: backupManifest.blobs.length, allRestoredBlobHashesVerified: true,
      resourcesRestored: backupManifest.resources?.length || 0, jobsRestored: backupManifest.jobs?.length || 0, stagedUploadsRestored: backupManifest.staging?.length || 0,
      scope: 'pg_dump consistent DB snapshot including fixture resources stored bytea; no external object store or SQLite import' };
  } finally { await restored.close(); assert.equal(gateways.entries, oldEntries); }
  } finally { await v2.close(); }
}
