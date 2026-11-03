// Standalone real-database checks; owns only its randomly named test project.
import assert from 'node:assert/strict';
import http from 'node:http';

const [a, b] = process.argv.slice(2);
assert(a && b, 'Usage: node tests/smoke.mjs http://gateway-a http://gateway-b');
const projectId = `pg_smoke_${Date.now()}`;
async function call(base, path, body, headers = {}) {
  const response = await fetch(`${base}${path}`, {
    method: body ? 'POST' : 'GET',
    headers: { 'content-type': 'application/json', ...headers },
    body: body ? JSON.stringify(body) : undefined,
  });
  return { status: response.status, body: await response.json() };
}
function batch(key, revision, operations, dryRun = false) {
  return { projectId, expectedRevision: revision, idempotencyKey: key, operations, dryRun };
}
const create = (id, parentId, kind) => ({ op: 'create_node', id, parentId, kind, props: {} });
assert.equal((await call(a, '/projects', { projectId })).status, 200);
assert.equal((await call(b, '/projects', { projectId })).status, 409);
const init = batch('init', 0, [create('p', null, 'page'), create('f', 'p', 'frame')]);
assert.equal((await call(a, '/batch', init)).body.committedRevision, 1);
assert.equal((await call(b, '/batch', init)).body.replayed, true);
const collision = structuredClone(init); collision.operations[1].props.width = 10;
assert.equal((await call(b, '/batch', collision)).body.error.code, 'IDEMPOTENCY_KEY_REUSED');
const invalid = batch('invalid', 1, [create('temp', 'p', 'group'), create('bad', 'missing', 'frame')]);
assert.equal((await call(b, '/batch', invalid)).status, 400);
assert.equal((await call(a, `/projects/${projectId}`)).body.nodes.length, 2);
const preview = batch('preview', 1, [{ op: 'update_node', id: 'f', props: { width: 22 } }], true);
assert.equal((await call(a, '/batch', preview)).body.dryRun, true);
assert.equal((await call(b, `/changes/${projectId}`)).body.commits.length, 1);

const abort = new AbortController();
const response = await fetch(`${b}/events/${projectId}?sinceRevision=0`, { signal: abort.signal });
const reader = response.body.getReader();
let sse = '';
const collect = (async () => {
  try { while (true) { const { value, done } = await reader.read(); if (done) break; sse += new TextDecoder().decode(value); } }
  catch (error) { if (error.name !== 'AbortError') throw error; }
})();
const contenders = await Promise.all([a, b].map((base, i) => call(base, '/batch', batch(`race_${i}`, 1, [{ op: 'update_node', id: 'f', props: { height: i + 10 } }]))));
assert.deepEqual(contenders.map(r => r.status).sort(), [200, 409]);
const lost = batch('lost', 2, [{ op: 'update_node', id: 'f', props: { x: 42 } }]);
await new Promise((resolve, reject) => {
  const data = JSON.stringify(lost);
  const req = http.request(`${a}/batch`, { method: 'POST', headers: { 'content-type': 'application/json', 'content-length': Buffer.byteLength(data), 'x-spike-drop-response': '1' } }, () => reject(new Error('Expected no response')));
  req.on('error', error => error.code === 'ECONNRESET' ? resolve() : reject(error));
  req.end(data);
});
const replay = await call(b, '/batch', lost);
assert.equal(replay.body.replayed, true);
assert.equal(replay.body.committedRevision, 3);
const until = Date.now() + 3000;
while (!sse.includes(`id: ${projectId}:3\n`) && Date.now() < until) await new Promise(r => setTimeout(r, 25));
abort.abort(); await collect;
assert.deepEqual([...sse.matchAll(new RegExp(`id: ${projectId}:(\\d+)`, 'g'))].map(m => +m[1]), [1, 2, 3]);
const log = await call(a, `/changes/${projectId}?sinceRevision=0`);
assert.deepEqual(log.body.commits.map(c => c.revision), [1, 2, 3]);
assert.equal((await call(b, `/projects/${projectId}`)).body.revision, 3);
assert.equal((await call(a, '/projects/missing')).status, 404);
console.log(JSON.stringify({ ok: true, projectId, checks: ['atomic rollback', 'dry-run', 'race conflict', 'collision', 'cross-gateway lost-response replay', 'cross-gateway SSE durable catch-up', 'gapless revision log'] }));
