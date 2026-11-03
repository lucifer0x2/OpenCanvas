// Same immutable single-writer workload for baseline / delta-only / delta+Redis.
import assert from 'node:assert/strict';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { root, docker, owned, sql, startEngine, stopEngine } from '../infra/runtime.mjs';
import { startRedis, stopRedis } from '../infra/redis-runtime.mjs';
import { Gateways } from '../infra/gateways.mjs';
import { request } from './client.mjs';
import { quantiles, singleWriterLoad } from './single-writer-load.mjs';
import { task17Measured, task17Profiles } from './task17-profiles.mjs';
import { freezeTask17, frozenTask17 } from './task17-provenance.mjs';

const stage = process.env.STAGE;
assert.ok(['baseline', 'delta-only', 'delta-redis'].includes(stage));
const source = resolve(process.env.SPIKE_SOURCE || root);
const binary = resolve(process.env.SPIKE_BINARY || resolve(source, 'postgres/target/debug/oc-spike-postgres'));
const destination = resolve(process.env.EVIDENCE_DIR);
mkdirSync(destination);
const configuration = { nodeCounts: [100, 5000], writers: 1, warmup: 20, samples: 100,
  buildProfile: 'cargo dev (unoptimized + debuginfo)', batchProfiling: true, buildDuringRun: false,
  client: 'unchanged singleWriterLoad/commitFixture; revision GET + POST 3-node updates; continuous reader + SSE',
  redisProvisionedAllStages: true, cache: { ttlSeconds: 3600, maxBytes: 8388608, timeoutMs: 100, enabled: stage === 'delta-redis' } };
const report = { stage, status: 'running', configuration, startedAt: new Date().toISOString(), cells: [] };
const save = () => writeFileSync(resolve(destination, 'report.json'), JSON.stringify(report, null, 2));
const raw = (name, records) => writeFileSync(resolve(destination, name), records.map(value => JSON.stringify(value)).join('\n') + '\n', { flag: 'wx' });
let pg, redis, gateways, volumes = [];
try {
  report.provenance = freezeTask17(source, binary, root); save();
  pg = await startEngine(); redis = await startRedis();
  report.topology = structuredClone(pg.manifest); report.redisTopology = structuredClone(redis.manifest);
  report.durability = { fsync: sql(pg, 'SHOW fsync').trim(), synchronousCommit: sql(pg, 'SHOW synchronous_commit').trim(),
    fullPageWrites: sql(pg, 'SHOW full_page_writes').trim() };
  assert.deepEqual(report.durability, { fsync: 'on', synchronousCommit: 'on', fullPageWrites: 'on' });
  volumes = owned(pg).Mounts.filter(value => value.Type === 'volume').map(value => value.Name);
  const env = { ...pg.env, OC_SPIKE_PROFILE_BATCH: '1', REDIS_CACHE_TTL_SECONDS: '3600',
    REDIS_CACHE_MAX_BYTES: '8388608', REDIS_CACHE_TIMEOUT_MS: '100' };
  // An inherited cache setting may never accidentally enable the off stages.
  env.REDIS_URL = stage === 'delta-redis' ? redis.env.REDIS_URL : '';
  gateways = new Gateways({ ...pg, env }, binary);
  report.startup = [await gateways.start(0), await gateways.start(1)];
  assert.equal(Number(sql(pg, 'SELECT count(*) FROM projects')), 0);
  for (const nodeCount of configuration.nodeCounts) {
    const before = freezeTask17(source, binary, root); frozenTask17(report.provenance, before);
    const log = gateways.entries[0].logPath, offset = task17Profiles(log).length;
    const metricBefore = await Promise.all(gateways.urls.map(url => request(url, '/health')));
    console.log(JSON.stringify({ stage, nodeCount, state: 'start' }));
    const cell = await singleWriterLoad(gateways.urls, nodeCount, 20, 100);
    const profiles = task17Profiles(log).slice(offset), measured = task17Measured(profiles, cell, stage !== 'baseline');
    raw(`${nodeCount}-attempts.jsonl`, cell.attempts); raw(`${nodeCount}-successes.jsonl`, cell.successes);
    raw(`${nodeCount}-reads.jsonl`, cell.reads); raw(`${nodeCount}-profiles-all.jsonl`, profiles);
    raw(`${nodeCount}-profiles-measured.jsonl`, measured.records);
    cell.continuousReadMs = quantiles(cell.reads.map(read => read.ms));
    delete cell.attempts; delete cell.successes; delete cell.reads;
    cell.profile = measured.summary; cell.metricsBefore = metricBefore.map(result => result.body.cacheMetrics ?? null);
    cell.metricsAfter = (await Promise.all(gateways.urls.map(url => request(url, '/health')))).map(result => result.body.cacheMetrics ?? null);
    cell.cacheMetrics = cell.metricsBefore[0] === null ? null : Object.fromEntries(Object.keys(cell.metricsBefore[0]).map(name =>
      [name, cell.metricsAfter.reduce((sum, row, index) => sum + row[name] - cell.metricsBefore[index][name], 0)]));
    cell.provenanceBefore = before; cell.provenanceAfter = freezeTask17(source, binary, root); frozenTask17(before, cell.provenanceAfter);
    report.cells.push(cell); save();
    console.log(JSON.stringify({ stage, nodeCount, state: 'pass', postMs: cell.postMs, wholeOperationMs: cell.wholeOperationMs }));
  }
  report.databaseCounts = Object.fromEntries(['projects', 'commits', 'idempotency'].map(table => [table, Number(sql(pg, `SELECT count(*) FROM ${table}`))]));
  assert.deepEqual(report.databaseCounts, { projects: 2, commits: 251, idempotency: 251 });
  report.provenanceAfter = freezeTask17(source, binary, root); frozenTask17(report.provenance, report.provenanceAfter); report.status = 'pass';
} catch (error) { report.status = 'fail'; report.error = { message: error.message, stack: error.stack }; process.exitCode = 1; }
finally {
  try {
    if (gateways) await gateways.close();
    if (pg) {
      stopEngine(pg);
      const absent = args => { try { docker(args); return false; } catch (error) { assert.equal(error.status, 1); return true; } };
      report.cleanup = { postgresContainerAbsent: absent(['inspect', pg.manifest.containerId]),
        postgresVolumesAbsent: volumes.every(name => absent(['volume', 'inspect', name])),
        gatewayProcessesAbsent: gateways.entries.every(({ pid }) => { try { process.kill(pid, 0); return false; } catch (error) { return error.code === 'ESRCH'; } }) };
      rmSync(pg.directory, { recursive: true }); report.cleanup.postgresCredentialsRemoved = true;
    }
    if (redis) report.redisCleanup = stopRedis(redis);
    assert.ok(Object.values(report.cleanup ?? {}).every(Boolean));
  } catch (error) { report.status = 'fail'; report.cleanupError = error.message; process.exitCode = 1; }
  report.finishedAt = new Date().toISOString(); save();
  console.log(JSON.stringify({ stage, status: report.status, evidence: destination, error: report.error?.message, cleanup: report.cleanup, redisCleanup: report.redisCleanup }));
}
