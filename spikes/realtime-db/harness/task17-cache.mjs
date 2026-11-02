import assert from 'node:assert/strict';
import { mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { root, startEngine, stopEngine, owned, docker } from '../infra/runtime.mjs';
import { startRedis, stopRedis } from '../infra/redis-runtime.mjs';
import { Gateways } from '../infra/gateways.mjs';
import { sourceManifest, sha256 } from './single-writer-provenance.mjs';
import { cacheScenarios } from './task17-cache-scenarios.mjs';
import { hotReadLoad } from './task17-hot-load.mjs';

const destination = resolve(process.env.EVIDENCE_DIR);
const binary = resolve(process.env.SPIKE_BINARY || resolve(root, 'postgres/target/debug/oc-spike-postgres'));
mkdirSync(destination);
const report = { status: 'running', startedAt: new Date().toISOString(), source: sourceManifest(root), binary: sha256(readFileSync(binary)),
  cache: { ttlSeconds: 3600, maxBytes: 8388608, timeoutMs: 100 }, buildDuringRun: false };
const save = () => writeFileSync(resolve(destination, 'report.json'), JSON.stringify(report, null, 2));
let pg, redis, gateways, off, volumes = [];
try {
  pg = await startEngine(); redis = await startRedis();
  report.topology = structuredClone(pg.manifest); report.redisTopology = structuredClone(redis.manifest);
  volumes = owned(pg).Mounts.filter(value => value.Type === 'volume').map(value => value.Name);
  const settings = { REDIS_CACHE_TIMEOUT_MS: '100', REDIS_CACHE_TTL_SECONDS: '3600', REDIS_CACHE_MAX_BYTES: '8388608' };
  gateways = new Gateways({ ...pg, env: { ...pg.env, ...redis.env, ...settings } }, binary);
  await gateways.start(0); await gateways.start(1);
  report.checks = await cacheScenarios(pg, redis, gateways.urls); save();
  // Sequential counterfactual: one writer count remains zero for these HOT reads.
  off = new Gateways({ ...pg, env: { ...pg.env, REDIS_URL: '', ...settings } }, binary);
  await off.start(0); await off.start(1);
  report.hotReadNoCache = await hotReadLoad(off.urls);
  await off.close();
  report.hotReadRedis = await hotReadLoad(gateways.urls);
  assert.deepEqual(report.source, sourceManifest(root), 'Code must remain frozen through lifecycle and HOT load');
  report.binaryAfter = sha256(readFileSync(binary)); assert.equal(report.binaryAfter, report.binary);
  report.status = 'pass';
} catch (error) { report.status = 'fail'; report.error = { message: error.message, stack: error.stack }; process.exitCode = 1; }
finally {
  try {
    if (off) await off.close(); if (gateways) await gateways.close();
    if (pg) {
      stopEngine(pg);
      const absent = args => { try { docker(args); return false; } catch (error) { assert.equal(error.status, 1); return true; } };
      report.cleanup = { postgresContainerAbsent: absent(['inspect', pg.manifest.containerId]),
        postgresVolumesAbsent: volumes.every(name => absent(['volume', 'inspect', name])),
        gatewayProcessesAbsent: [...(gateways?.entries || []), ...(off?.entries || [])].every(({ pid }) => {
          try { process.kill(pid, 0); return false; } catch (error) { return error.code === 'ESRCH'; }
        }) };
      rmSync(pg.directory, { recursive: true }); report.cleanup.postgresCredentialsRemoved = true;
    }
    if (redis) report.redisCleanup = stopRedis(redis);
    assert.ok(Object.values(report.cleanup ?? {}).every(Boolean));
  } catch (error) { report.status = 'fail'; report.cleanupError = error.message; process.exitCode = 1; }
  report.finishedAt = new Date().toISOString(); save();
  console.log(JSON.stringify({ status: report.status, evidence: destination, error: report.error?.message, cleanup: report.cleanup, redisCleanup: report.redisCleanup }));
}
