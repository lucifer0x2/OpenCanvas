import assert from 'node:assert/strict';
import { batch, create, key, pause, request, subscribe, update } from './client.mjs';

export async function semantics(urls) {
  const [a, b] = urls;
  const projectId = `sem-${key()}`;
  const checks = [];
  const check = async (name, fn) => { await fn(); checks.push({ name, status: 'pass' }); };
  assert.equal((await request(a, '/projects', { projectId })).status, 200);
  const state = async (url = b) => (await request(url, `/projects/${projectId}`)).body;
  await check('invalid late operation rolls back nodes, revision, log and idempotency', async () => {
    const invalid = batch(projectId, 0, [create('root'), create('bad', 'missing', 'text')]);
    assert.equal((await request(a, '/batch', invalid)).status, 400);
    assert.deepEqual((await state()).nodes, []);
    assert.equal((await state()).revision, 0);
    assert.equal((await request(b, `/changes/${projectId}?sinceRevision=0`)).body.commits.length, 0);
    const corrected = { ...invalid, operations: [create('root')] };
    assert.equal((await request(b, '/batch', corrected)).status, 200);
  });
  await check('same revision race commits exactly once across two gateway processes', async () => {
    const responses = await Promise.all(urls.map((url, i) => request(url, '/batch', batch(projectId, 1, [update('root', { name: `race-${i}` })]))));
    assert.deepEqual(responses.map(r => r.status).sort(), [200, 409]);
    assert.equal((await state()).revision, 2);
  });
  await check('lost ACK replay on other gateway and key collision', async () => {
    const lost = batch(projectId, 2, [update('root', { name: 'lost-ack' })]);
    await assert.rejects(request(a, '/batch', lost, { 'x-spike-drop-response': '1' }));
    const replay = await request(b, '/batch', lost);
    assert.equal(replay.status, 200);
    assert.equal(replay.body.committedRevision, 3);
    assert.equal(replay.body.replayed, true);
    assert.equal((await state()).revision, 3);
    assert.equal((await request(a, '/batch', { ...lost, operations: [update('root', { name: 'collision' })] })).status, 409);
  });
  const stream = subscribe(b, projectId, 3);
  await stream.ready;
  try {
    await check('dry run does not reserve key, change revision or publish event', async () => {
      const dry = batch(projectId, 3, [update('root', { name: 'dry' })], { dryRun: true });
      const result = await request(a, '/batch', dry);
      assert.equal(result.status, 200);
      assert.equal(result.body.dryRun, true);
      await pause(150);
      assert.equal(stream.events.length, 0);
      assert.equal((await state()).revision, 3);
      assert.equal((await request(b, '/batch', { ...dry, dryRun: false })).status, 200);
    });
    await check('cross gateway subscriptions ordered and gapless', async () => {
      for (let rev = 4; rev < 8; rev++) {
        assert.equal((await request(urls[rev % 2], '/batch', batch(projectId, rev, [update('root', { name: `stream-${rev}` })]))).status, 200);
      }
      const events = await stream.until(5);
      assert.deepEqual(events.map(e => e.revision), [4, 5, 6, 7, 8]);
    });
  } finally { await stream.close(); }
  await check('disconnect reconnect replays durable commits exactly once', async () => {
    for (let rev = 8; rev < 11; rev++) assert.equal((await request(a, '/batch', batch(projectId, rev, [update('root', { name: `offline-${rev}` })]))).status, 200);
    const reconnected = subscribe(a, projectId, 8);
    try { assert.deepEqual((await reconnected.until(3)).map(e => e.revision), [9, 10, 11]); }
    finally { await reconnected.close(); }
  });
  await check('partial update failure and cycle leave previous snapshot intact', async () => {
    const before = await state();
    const result = await request(a, '/batch', batch(projectId, 11, [update('root', { name: 'must-rollback' }), create('child', 'root', 'group'), { op: 'reparent_node', id: 'root', parentId: 'child' }]));
    assert.equal(result.status, 400);
    assert.deepEqual(await state(), before);
  });
  await check('project scoped node IDs and idempotency keys', async () => {
    const other = `other-${key()}`;
    await request(a, '/projects', { projectId: other });
    const sharedKey = key();
    assert.equal((await request(a, '/batch', batch(other, 0, [create('root')], { idempotencyKey: sharedKey }))).status, 200);
    assert.equal((await request(b, '/batch', batch(projectId, 11, [update('root', { name: 'independent' })], { idempotencyKey: sharedKey }))).status, 200);
    assert.equal((await request(b, `/projects/${other}`)).body.revision, 1);
  });
  return { projectId, checks, finalRevision: (await state()).revision };
}
