import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { performance } from 'node:perf_hooks';
import { docker, sql } from '../infra/runtime.mjs';
import { pause, request } from './client.mjs';
import { redisProbe } from './redis-probe.mjs';
import { quantiles } from './single-writer-load.mjs';

export async function metrics(urls) {
  return (await Promise.all(urls.map(url => request(url, '/health')))).map(result => {
    assert.equal(result.status, 200); assert.ok(result.body.cacheMetrics); return result.body.cacheMetrics;
  });
}
export function metricDelta(before, after) {
  return Object.fromEntries(Object.keys(before[0]).map(name => [name,
    after.reduce((sum, row, index) => sum + row[name] - before[index][name], 0)]));
}
export async function cacheRead(url, projectId, revision) {
  const result = await request(url, `/projects/${projectId}`, undefined, {}, 3000);
  assert.equal(result.status, 200, 'Snapshot HTTP must succeed'); assert.equal(result.body.revision, revision); return result;
}
export async function hotReads(urls, projectId, revision, count = 100) {
  const before = await metrics(urls), reads = [];
  for (let n = 0; n < count; n++) reads.push((await cacheRead(urls[n % 2], projectId, revision)).ms);
  const delta = metricDelta(before, await metrics(urls));
  assert.equal(delta.hits, count); assert.equal(delta.nodeQueries, 0);
  return { count, latencyMs: quantiles(reads), metrics: delta, rawMs: reads };
}
export async function nodesLockProof(pg, redisUrl, entry, url, projectId, revision) {
  const tag = `task17-lock-${Date.now()}`;
  const child = spawn(process.env.DOCKER || '/usr/local/bin/docker', ['exec', '-i', pg.manifest.containerId,
    'psql', '-X', '-v', 'ON_ERROR_STOP=1', '-U', 'postgres', '-d', 'spike', '-At'], { stdio: ['pipe', 'ignore', 'ignore'] });
  child.stdin.end(`SET application_name='${tag}'; BEGIN; LOCK TABLE nodes IN ACCESS EXCLUSIVE MODE; SELECT pg_sleep(30); ROLLBACK;`);
  let pid, cold;
  try {
    for (let n = 0; n < 100; n++) {
      pid = Number(sql(pg, `SELECT pid FROM pg_stat_activity WHERE application_name='${tag}' AND wait_event='PgSleep'`).trim());
      if (pid) break; await pause(20);
    }
    assert.ok(pid, 'Exclusive nodes lock must actually be held');
    const result = await cacheRead(url, projectId, revision);
    assert.ok(result.ms < 1000, 'HOT read must not wait for nodes table');
    await redisProbe(redisUrl, ['DEL', entry]);
    // Removing exactly this key gives a real uncached HTTP positive control.
    let finished = false;
    cold = cacheRead(url, projectId, revision).finally(() => { finished = true; });
    let blocked = false;
    for (let n = 0; n < 100 && !finished; n++) {
      blocked = Number(sql(pg, `SELECT count(*) FROM pg_stat_activity WHERE wait_event_type='Lock' AND query LIKE 'SELECT node_id,% FROM nodes WHERE project_id=%'`).trim()) > 0;
      if (blocked) break; await pause(20);
    }
    assert.equal(blocked, true, 'Uncached HTTP read must actually block on nodes');
    assert.equal(finished, false);
    sql(pg, `SELECT pg_terminate_backend(${pid})`); pid = null;
    const uncached = await cold;
    return { exclusiveNodesLockHeld: true, status: result.status, ms: result.ms,
      uncachedPositiveControl: { nodesLockWaitObserved: blocked, status: uncached.status, ms: uncached.ms } };
  } finally {
    if (pid) sql(pg, `SELECT pg_terminate_backend(${pid})`);
    child.kill('SIGTERM');
    await new Promise(ok => child.exitCode !== null ? ok() : child.once('exit', ok));
    if (cold) await cold;
  }
}
export async function forceEviction(redis, key) {
  const url = redis.env.REDIS_URL, payload = 'x'.repeat(128 * 1024);
  const start = performance.now();
  const evicted = info => Number(info.match(/(?:^|\r\n)evicted_keys:(\d+)/)?.[1] || 0);
  const before = evicted(await redisProbe(url, ['INFO', 'stats']));
  await redisProbe(url, ['CONFIG', 'SET', 'maxmemory', 4 * 1024 * 1024]);
  let written = 0;
  try {
    // Avoid refreshing the HOT key's LRU access on every filler insertion.
    while (written < 256 && await redisProbe(url, ['EXISTS', key])) {
      for (let n = 0; n < 32; n++, written++) await redisProbe(url, ['SET', `task17-eviction-${written}`, payload, 'EX', 60]);
    }
    assert.equal(await redisProbe(url, ['EXISTS', key]), 0, 'Actual LRU eviction must remove the HOT entry');
    const after = evicted(await redisProbe(url, ['INFO', 'stats']));
    assert.ok(after > before); return { evictedKeys: after - before, fillerKeys: written, ms: performance.now() - start };
  } finally { await redisProbe(url, ['CONFIG', 'SET', 'maxmemory', 64 * 1024 * 1024]); }
}
export function stopPostgres(pg) { docker(['stop', '-t', '2', pg.manifest.containerId]); }
export async function startPostgres(pg) {
  docker(['start', pg.manifest.containerId]);
  for (let n = 0; n < 100; n++) { try { sql(pg, 'SELECT 1'); return; } catch { await pause(100); } }
  throw new Error('Owned PostgreSQL restart timeout');
}
