import { test } from 'node:test';
import assert from 'node:assert/strict';
import { commitFixture, retryDelay } from './retry.mjs';

const revision = value => ({ status: 200, body: { projectId: 'p', revision: value }, ms: 1 });
const conflict = { status: 409, body: { error: { code: 'REVISION_CONFLICT' } }, ms: 2 };
const success = { status: 200, body: { committedRevision: 9 }, ms: 3 };
const options = { url: 'fixture', projectId: 'p', operations: [{ op: 'known-fixture' }] };

test('bounded jitter stays nonzero, grows and caps independently per retry', () => {
  assert.equal(retryDelay(0, () => 0), 1);
  assert.equal(retryDelay(1, () => 1), 4);
  assert.equal(retryDelay(1000, () => 0), 32);
  assert.equal(retryDelay(1000, () => 1), 64);
});

test('only explicit revision conflict rebases fixture with new revision/key after waiting', async () => {
  const calls = [], sleeps = [], attempts = [];
  const outcomes = [revision(6), conflict, revision(8), success];
  const result = await commitFixture({ ...options, random: () => 0, sleep: async ms => sleeps.push(ms),
    send: async (_url, path, body) => { calls.push({ path, body }); return outcomes.shift(); },
    onAttempt: value => attempts.push(value) });
  assert.deepEqual(calls.map(value => value.path), ['/projects/p/revision', '/batch', '/projects/p/revision', '/batch']);
  assert.deepEqual(calls.filter(value => value.body).map(value => value.body.expectedRevision), [6, 8]);
  assert.notEqual(calls[1].body.idempotencyKey, calls[3].body.idempotencyKey);
  assert.deepEqual(sleeps, [1]);
  assert.equal(result.retries, 1); assert.equal(result.revisionReads, 2); assert.equal(result.revision, 9);
  assert.deepEqual(attempts.map(value => value.status), [409, 200]);
});

test('lost POST response retries identical payload/key without a new revision read', async () => {
  const bodies = [], paths = [], sleeps = [];
  let posts = 0;
  const result = await commitFixture({ ...options, random: () => 0, sleep: async ms => sleeps.push(ms),
    send: async (_url, path, body) => {
      paths.push(path);
      if (!body) return revision(8);
      bodies.push(body);
      if (++posts === 1) throw new Error('response lost after commit');
      return success;
    } });
  assert.deepEqual(paths, ['/projects/p/revision', '/batch', '/batch']);
  assert.strictEqual(bodies[0], bodies[1]); assert.deepEqual(sleeps, [1]);
  assert.equal(result.networkRetries, 1); assert.equal(result.retries, 0);
});

test('collision, validation failure and HTTP server errors do not rebase or retry', async () => {
  for (const [status, code] of [[409, 'IDEMPOTENCY_COLLISION'], [422, 'VALIDATION_FAILED'], [500, 'INTERNAL']]) {
    let calls = 0;
    await assert.rejects(commitFixture({ ...options, sleep: async () => assert.fail('unexpected backoff'),
      send: async () => ++calls === 1 ? revision(8) : { status, body: { error: { code } }, ms: 1 } }));
    assert.equal(calls, 2);
  }
});

test('conflict and uncertain outcome bounds fail with finite attempts', async () => {
  let posts = 0;
  await assert.rejects(commitFixture({ ...options, policy: { maxConflicts: 1, maxNetworkRetries: 1, baseMs: 2, capMs: 4 },
    sleep: async () => {}, send: async (_url, _path, body) => body ? (posts++, conflict) : revision(8) }), /bound/);
  assert.equal(posts, 2);
  posts = 0;
  await assert.rejects(commitFixture({ ...options, policy: { maxConflicts: 1, maxNetworkRetries: 1, baseMs: 2, capMs: 4 },
    sleep: async () => {}, send: async (_url, _path, body) => {
      if (!body) return revision(8); posts++; throw new Error('offline');
    } }), /offline/);
  assert.equal(posts, 2);
});
