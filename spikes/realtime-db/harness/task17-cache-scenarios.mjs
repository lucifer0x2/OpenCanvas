import assert from 'node:assert/strict';
import { batch, create, key, pause, request, update } from './client.mjs';
import { fixture } from './load.mjs';
import { redisProbe } from './redis-probe.mjs';
import { pauseRedis, restartRedis, resumeRedis } from '../infra/redis-runtime.mjs';
import { sql } from '../infra/runtime.mjs';
import { quantiles } from './single-writer-load.mjs';
import { cacheRead, forceEviction, hotReads, metricDelta, metrics, nodesLockProof, startPostgres, stopPostgres } from './task17-cache-assertions.mjs';

export async function cacheScenarios(pg, redis, urls) {
  const checks = {}, redisUrl = redis.env.REDIS_URL;
  const { projectId, revision: initial } = await fixture(urls[0], 100);
  let revision = initial;
  await redisProbe(redisUrl, ['FLUSHDB']);
  let before = await metrics(urls);
  const cold = await cacheRead(urls[0], projectId, revision);
  let delta = metricDelta(before, await metrics(urls));
  assert.equal(delta.misses, 1); assert.equal(delta.nodeQueries, 1); assert.equal(delta.stores, 1);
  checks.cold = { status: cold.status, ms: cold.ms, metrics: delta };
  checks.hot = await hotReads(urls, projectId, revision);
  const entries = await redisProbe(redisUrl, ['KEYS', 'oc:snapshot:*']);
  assert.equal(entries.length, 1);
  const entry = entries[0], ttl = await redisProbe(redisUrl, ['TTL', entry]);
  assert.ok(ttl > 0 && ttl <= 3600); checks.ttlSeconds = ttl;
  checks.nodesLock = await nodesLockProof(pg, redisUrl, entry, urls[1], projectId, revision);
  await redisProbe(redisUrl, ['SET', entry, '{malformed', 'EX', 3600]);
  before = await metrics(urls); await cacheRead(urls[0], projectId, revision);
  delta = metricDelta(before, await metrics(urls)); assert.equal(delta.malformed, 1); assert.equal(delta.nodeQueries, 1);
  checks.malformed = delta;
  const oversizeBytes = 8388608 + 37;
  await redisProbe(redisUrl, ['SET', entry, 'x'.repeat(oversizeBytes), 'EX', 3600]);
  assert.equal(await redisProbe(redisUrl, ['STRLEN', entry]), oversizeBytes);
  before = await metrics(urls); await cacheRead(urls[0], projectId, revision);
  delta = metricDelta(before, await metrics(urls));
  assert.equal(delta.misses, 1); assert.equal(delta.malformed, 0); assert.equal(delta.nodeQueries, 1); assert.equal(delta.stores, 1);
  assert.ok(await redisProbe(redisUrl, ['STRLEN', entry]) <= 8388608 + 36);
  checks.oversizedEntry = { storedBytes: oversizeBytes, gatewayBodyLimitBytes: 8388608, metrics: delta };
  checks.eviction = await forceEviction(redis, entry);
  before = await metrics(urls); await cacheRead(urls[1], projectId, revision);
  delta = metricDelta(before, await metrics(urls)); assert.equal(delta.misses, 1); assert.equal(delta.nodeQueries, 1);
  checks.eviction.fallbackMetrics = delta;
  await restartRedis(redis);
  assert.equal(await redisProbe(redisUrl, ['DBSIZE']), 0);
  before = await metrics(urls); await cacheRead(urls[0], projectId, revision);
  delta = metricDelta(before, await metrics(urls)); assert.equal(delta.nodeQueries, 1);
  checks.restart = { emptyAfterRestart: true, metrics: delta };
  await redisProbe(redisUrl, ['CLIENT', 'PAUSE', 800, 'ALL']);
  before = await metrics(urls);
  const timedOut = await cacheRead(urls[0], projectId, revision);
  assert.ok(timedOut.ms < 1500);
  delta = metricDelta(before, await metrics(urls)); assert.ok(delta.fallbacks > 0); assert.equal(delta.nodeQueries, 1);
  checks.timeout = { pauseMs: 800, ms: timedOut.ms, metrics: delta };
  await pause(900);
  pauseRedis(redis);
  try {
    before = await metrics(urls);
    const fallback = await cacheRead(urls[0], projectId, revision);
    assert.ok(fallback.ms < 1500); checks.unavailable = { ms: fallback.ms };
    const body = batch(projectId, revision, [update('n-3', { text: 'Actual text edit', fill: '#ff0000', width: 200 })]);
    const beforeWrite = await metrics(urls);
    const result = await request(urls[0], '/batch', body); assert.equal(result.status, 200); revision++;
    assert.equal(result.body.committedRevision, revision);
    const writeMetrics = metricDelta(beforeWrite, await metrics(urls));
    assert.ok(Object.values(writeMetrics).every(value => value === 0), 'ACK path must perform zero cache operations');
    const other = await cacheRead(urls[1], projectId, revision);
    assert.equal(other.body.nodes.find(node => node.id === 'n-3').props.fill, '#ff0000');
    checks.unavailable.commitMs = result.ms; checks.unavailable.writeMetrics = writeMetrics;
    checks.unavailable.metrics = metricDelta(before, await metrics(urls));
  } finally { await resumeRedis(redis); }
  // A real dropped ACK is retried through the second gateway with the same key/body.
  const lost = batch(projectId, revision, [update('n-3', { text: 'Lost response durable text' })]);
  const countBefore = Number(sql(pg, 'SELECT count(*) FROM commits'));
  await assert.rejects(request(urls[0], '/batch', lost, { 'x-spike-drop-response': '1' }));
  const replay = await request(urls[1], '/batch', lost); assert.equal(replay.status, 200); assert.equal(replay.body.replayed, true); revision++;
  assert.equal(Number(sql(pg, 'SELECT count(*) FROM commits')), countBefore + 1);
  checks.lostResponse = { replayed: true, commitsAdded: 1, revision: replay.body.committedRevision };
  // Keep prior cached revisions present. DB identity must expose each later commit.
  const bursts = [];
  for (const readsPerCommit of [1, 10]) {
    before = await metrics(urls); const latencies = [];
    for (let n = 0; n < 10; n++) {
      const result = await request(urls[n % 2], '/batch', batch(projectId, revision, [update('n-3', { text: `Burst ${readsPerCommit}/${n}`, fill: n % 2 ? '#00ff00' : '#ff0000' })]));
      assert.equal(result.status, 200); revision++;
      for (let read = 0; read < readsPerCommit; read++) latencies.push((await cacheRead(urls[read % 2], projectId, revision)).ms);
    }
    const burstMetrics = metricDelta(before, await metrics(urls));
    assert.equal(burstMetrics.nodeQueries, 10); assert.equal(burstMetrics.hits, 10 * (readsPerCommit - 1));
    bursts.push({ readsPerCommit, commits: 10, reads: latencies.length, metrics: burstMetrics, latencyMs: quantiles(latencies), rawMs: latencies });
  }
  checks.bursts = bursts;
  // Structural fallback and rejection cannot change the previously cached result.
  const structural = await request(urls[0], '/batch', batch(projectId, revision, [create('structural', 'root', 'text', { text: 'New structural node' })]));
  assert.equal(structural.status, 200); revision++;
  const expected = (await cacheRead(urls[1], projectId, revision)).body;
  const rejected = await request(urls[0], '/batch', batch(projectId, revision, [update('n-3', { width: -1 })]));
  assert.equal(rejected.status, 400);
  assert.deepEqual((await cacheRead(urls[1], projectId, revision)).body, expected);
  checks.structuralAndRollback = { structuralRevision: revision, rejectedStatus: rejected.status };
  stopPostgres(pg);
  try {
    const result = await request(urls[1], `/projects/${projectId}`, undefined, {}, 5000);
    assert.equal(result.status, 500); checks.databaseOutage = { status: result.status, ms: result.ms, cachedSuccessRefused: true };
  } finally { await startPostgres(pg); }
  await cacheRead(urls[1], projectId, revision);
  // Recreate a second project at an equal cached revision to prove generation isolation.
  const reused = `recreate-${key()}`;
  assert.equal((await request(urls[0], '/projects', { projectId: reused })).status, 200);
  assert.equal((await request(urls[0], '/batch', batch(reused, 0, [create('old')]))).status, 200);
  await cacheRead(urls[0], reused, 1);
  const oldGeneration = sql(pg, `SELECT generation FROM projects WHERE project_id='${reused}'`).trim();
  sql(pg, `BEGIN; DELETE FROM idempotency WHERE project_id='${reused}'; DELETE FROM commits WHERE project_id='${reused}'; DELETE FROM nodes WHERE project_id='${reused}'; DELETE FROM projects WHERE project_id='${reused}'; COMMIT;`);
  assert.equal((await request(urls[0], '/projects', { projectId: reused })).status, 200);
  assert.equal((await request(urls[0], '/batch', batch(reused, 0, [create('new')]))).status, 200);
  const recreated = await cacheRead(urls[1], reused, 1);
  assert.deepEqual(recreated.body.nodes.map(node => node.id), ['new']);
  assert.notEqual(sql(pg, `SELECT generation FROM projects WHERE project_id='${reused}'`).trim(), oldGeneration);
  checks.recreatedId = { reusedProjectId: true, sameRevision: 1, generationChanged: true, nodes: ['new'] };
  checks.elapsedFinishedAt = new Date().toISOString();
  return checks;
}
