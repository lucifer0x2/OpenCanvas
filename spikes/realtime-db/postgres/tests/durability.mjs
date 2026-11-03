// Real two-gateway DB fixture; lifecycle belongs to the outer harness.
import assert from 'node:assert/strict';
import { createHash, randomUUID } from 'node:crypto';
import { fork } from 'node:child_process';
import { once } from 'node:events';
import { pathToFileURL } from 'node:url';
import { request, pause, batch, create, update } from '../../harness/client.mjs';

const hash = bytes => createHash('sha256').update(Buffer.from(bytes)).digest('hex');
const output = text => [...Buffer.from(text)];

async function claimAndKillWorker(base) {
  const child = fork(new URL('./fixture-worker.mjs', import.meta.url), [base], {
    stdio: ['ignore', 'ignore', 'pipe', 'ipc'],
  });
  let stderr = '';
  let timeout;
  child.stderr.on('data', bytes => { stderr += bytes.toString(); });
  try {
    const [message] = await Promise.race([
      once(child, 'message'),
      once(child, 'exit').then(([code]) => { throw new Error(`Worker exited ${code}: ${stderr}`); }),
      new Promise((_, reject) => {
        timeout = setTimeout(() => reject(new Error('Worker claim IPC timeout')), 10000);
      }),
    ]);
    assert(!message.error, message.error);
    const exited = once(child, 'exit');
    assert(child.kill('SIGKILL'));
    const [code, signal] = await exited;
    assert.equal(code, null);
    assert.equal(signal, 'SIGKILL');
    return message.job;
  } finally {
    clearTimeout(timeout);
    if (child.exitCode === null && child.signalCode === null) child.kill('SIGKILL');
  }
}

export async function exercise(urls, lifecycle = {}) {
  const [a, b] = urls;
  const projectId = `durable_${randomUUID().replaceAll('-', '')}`;
  const call = async (base, path, body) => {
    try { return await request(base, path, body); }
    catch (cause) { throw new Error(`Durability request failed at ${path}: ${cause.message}`, { cause }); }
  };
  const ok = async (base, path, body) => {
    const response = await call(base, path, body);
    assert.equal(response.status, 200, JSON.stringify(response.body));
    return response.body;
  };
  const error = async (base, path, body, code, status = 400) => {
    const response = await call(base, path, body);
    assert.equal(response.status, status, JSON.stringify(response.body));
    assert.equal(response.body.error.code, code);
  };
  await ok(a, '/projects', { projectId });
  const fixtureBytes = { asset: [137, 80, 78, 71, 0, 255, 12], font: [0, 1, 0, 0, 255, 77] };
  const resources = [];
  for (const [kind, bytes] of Object.entries(fixtureBytes)) {
    const resource = await ok(a, '/resources', { kind, bytes, sha256: hash(bytes) });
    assert.equal(resource.resourceId, hash(bytes));
    assert.deepEqual((await ok(b, `/resources/${resource.resourceId}`)).bytes, bytes);
    resources.push(resource.resourceId);
  }
  await error(b, '/resources', { kind: 'font', bytes: [0], sha256: 'wrong' }, 'VALIDATION_FAILED');

  const inline = batch(projectId, 0, [create('page')]);
  const preview = { ...inline, idempotencyKey: 'preview', dryRun: true };
  const previewUpload = await ok(a, '/uploads', preview);
  const ref = uploadId => ({ projectId, uploadId });
  assert.equal((await ok(b, '/batch-file', ref(previewUpload.uploadId))).dryRun, true);
  assert.equal((await ok(a, `/projects/${projectId}`)).revision, 0);
  assert.equal((await ok(b, `/changes/${projectId}`)).commits.length, 0);
  assert.equal((await ok(a, '/jobs/claim', { workerId: 'dryrun-check', leaseMs: 1000 })).job, null);
  const uploaded = await ok(a, '/uploads', inline);
  await error(b, '/batch-file', { projectId: 'other', uploadId: uploaded.uploadId }, 'NOT_FOUND', 404);
  await error(b, '/batch-file', { projectId, uploadId: '/tmp/pod-local.json' }, 'NOT_FOUND', 404);
  assert.equal((await ok(b, '/batch-file', ref(uploaded.uploadId))).committedRevision, 1);
  assert.equal((await ok(a, '/batch', inline)).replayed, true);
  // Dry-run did not reserve its key: a subsequent real batch with that key commits.
  const afterPreview = batch(projectId, 1, [update('page', { width: 1 })], { idempotencyKey: 'preview' });
  assert.equal((await ok(a, '/batch', afterPreview)).committedRevision, 2);
  const tooMany = batch(projectId, 2, Array.from({ length: 501 }, () => update('page', {})));
  await error(a, '/uploads', tooMany, 'VALIDATION_FAILED');
  const huge = batch(projectId, 2, [update('page', { text: 'x'.repeat(2 * 1024 * 1024) })]);
  await error(a, '/uploads', huge, 'VALIDATION_FAILED');

  const specification = revision => ({ projectId, revision, pipelineFingerprint: `pipeline-${revision}`,
    resourceIds: resources, renderParams: { frame: 'page', width: 100, height: 100 } });
  await error(a, '/jobs', specification(0), 'REVISION_CONFLICT', 409);
  const pending = await ok(a, '/jobs', specification(2));
  assert.equal(pending.status, 'pending');
  assert.equal((await ok(b, '/jobs', specification(2))).jobId, pending.jobId);
  const lifecyclePerformed = !!lifecycle.killGateway;
  if (lifecyclePerformed) await lifecycle.killGateway(0);
  assert.equal((await ok(b, `/jobs/${pending.jobId}`)).status, 'pending');
  const abandoned = await claimAndKillWorker(b);
  assert.equal(abandoned.jobId, pending.jobId);
  assert.equal(abandoned.attempt, 1);
  if (lifecyclePerformed) await lifecycle.restartGateway(0);
  await pause(350);
  const completion = (job, workerId, bytes) => ({ workerId, attempt: job.attempt, bytes });
  await error(b, `/jobs/${pending.jobId}/complete`, completion(abandoned, 'worker-dead', [1]), 'STALE_ATTEMPT', 409);
  const retried = (await ok(a, '/jobs/claim', { workerId: 'worker-live', leaseMs: 60000 })).job;
  assert.equal(retried.jobId, pending.jobId);
  assert.equal(retried.attempt, 2);
  await error(b, `/jobs/${pending.jobId}/complete`, completion(abandoned, 'worker-dead', [2]), 'STALE_ATTEMPT', 409);
  await error(b, `/jobs/${pending.jobId}/complete`, completion(retried, 'wrong-worker', [3]), 'STALE_ATTEMPT', 409);

  await ok(b, '/batch', batch(projectId, 2, [update('page', { width: 20 })]));
  const newer = await ok(b, '/jobs', specification(3));
  assert.notEqual(newer.jobId, pending.jobId);
  const newerClaim = (await ok(b, '/jobs/claim', { workerId: 'worker-new', leaseMs: 60000 })).job;
  assert.equal(newerClaim.jobId, newer.jobId);
  const newBytes = output('fixture-output-newer-revision-pipeline');
  const oldBytes = output('fixture-output-older-revision-pipeline');
  const newResult = await ok(b, `/jobs/${newer.jobId}/complete`, completion(newerClaim, 'worker-new', newBytes));
  const oldResult = await ok(a, `/jobs/${pending.jobId}/complete`, completion(retried, 'worker-live', oldBytes));
  assert.notEqual(newResult.artifactId, oldResult.artifactId);
  await error(a, `/jobs/${pending.jobId}/complete`, completion(retried, 'worker-live', [4]), 'STALE_ATTEMPT', 409);
  if (lifecyclePerformed) await lifecycle.killGateway(0);
  const pinned = await ok(b, `/jobs/${pending.jobId}`);
  assert.equal(pinned.input.snapshot.revision, 2);
  assert.equal(pinned.input.snapshot.nodes[0].props.width, 1);
  assert.equal(pinned.pipelineFingerprint, 'pipeline-2');
  assert.equal((await ok(b, `/jobs/${newer.jobId}`)).artifactId, newResult.artifactId);
  for (const [artifactId, bytes] of [[oldResult.artifactId, oldBytes], [newResult.artifactId, newBytes]]) {
    const artifact = await ok(b, `/artifacts/${artifactId}`);
    assert.equal(artifact.sha256, hash(bytes));
    assert.deepEqual(artifact.bytes, bytes);
  }
  for (const resourceId of resources) {
    const resource = await ok(b, `/resources/${resourceId}`);
    assert.equal(hash(resource.bytes), resourceId);
  }
  if (lifecyclePerformed) await lifecycle.restartGateway(0);
  return { ok: true, projectId, lifecyclePerformed, workerKilled: 'SIGKILL', resourceIds: resources, uploadId: uploaded.uploadId,
    jobIds: [pending.jobId, newer.jobId], artifactIds: [oldResult.artifactId, newResult.artifactId],
    checks: ['shared resources and verified bytes', 'bounded project-scoped JSON staging', 'inline replay',
      'staged dry-run has no revision/commit/job/key reservation', 'pending survives gateway death',
      'real claimed worker SIGKILL and expired lease retry', 'attempt and owner fencing', 'immutable pinned input',
      'older pipeline/revision cannot replace newer output', 'shared artifact retrieval after gateway death'] };
}

if (process.argv[1] && import.meta.url === pathToFileURL(process.argv[1]).href) {
  const urls = process.argv.slice(2);
  assert.equal(urls.length, 2, 'Usage: node tests/durability.mjs <gateway-a> <gateway-b>');
  console.log(JSON.stringify(await exercise(urls)));
}
