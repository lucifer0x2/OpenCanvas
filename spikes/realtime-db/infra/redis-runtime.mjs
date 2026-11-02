// Disposable cache fixture only: never discover, reuse or mutate inherited Redis.
import assert from 'node:assert/strict';
import { randomBytes } from 'node:crypto';
import { createServer } from 'node:net';
import { chmodSync, mkdirSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { docker, root } from './runtime.mjs';
import { pause } from '../harness/client.mjs';

export const redisImage = 'redis:7.4.2@sha256:fbdbaea47b9ae4ecc2082ecdb4e1cea81e32176ffb1dcf643d422ad07427e5d9';
export function ownedRedis(runtime) {
  const value = JSON.parse(docker(['inspect', runtime.manifest.containerId]))[0];
  assert.equal(value.Config.Labels['opencanvas.task17.redis'], runtime.manifest.runId, 'Redis ownership mismatch');
  return value;
}
export function redisCli(runtime, ...args) {
  ownedRedis(runtime);
  return docker(['exec', '-e', `REDISCLI_AUTH=${runtime.password}`, runtime.manifest.containerId,
    'redis-cli', '--no-auth-warning', '--raw', ...args]);
}
export async function readyRedis(runtime) {
  for (let n = 0; n < 100; n++) {
    try { if (redisCli(runtime, 'PING').trim() === 'PONG') return; } catch { /* Startup only. */ }
    await pause(100);
  }
  throw new Error('Owned Redis readiness timeout');
}
export async function startRedis() {
  const runId = `redis-task17-${Date.now()}-${randomBytes(4).toString('hex')}`;
  const directory = resolve(root, '.runtime', runId);
  mkdirSync(directory, { recursive: true, mode: 0o700 }); chmodSync(directory, 0o700);
  const password = randomBytes(24).toString('hex');
  const server = createServer();
  await new Promise((ok, fail) => { server.once('error', fail); server.listen(0, '127.0.0.1', ok); });
  const port = server.address().port;
  await new Promise(ok => server.close(ok));
  let containerId;
  try {
    containerId = docker(['run', '-d', '--name', runId, '--label', `opencanvas.task17.redis=${runId}`,
      '--cpus', '1', '--memory', '256m', '--memory-swap', '256m', '-p', `127.0.0.1:${port}:6379`,
      redisImage, 'redis-server', '--save', '', '--appendonly', 'no', '--requirepass', password,
      '--maxmemory', '64mb', '--maxmemory-policy', 'allkeys-lru']).trim();
    const inspection = JSON.parse(docker(['inspect', containerId]))[0];
    const env = { REDIS_URL: `redis://:${password}@127.0.0.1:${port}/0` };
    const manifest = { runId, createdAt: new Date().toISOString(), containerId, image: redisImage,
      imageId: inspection.Image, port, cpu: 1, memoryBytes: 268435456, maxmemoryBytes: 67108864,
      persistence: 'disabled (save empty; appendonly no)', eviction: 'allkeys-lru', loopbackOnly: true,
      volumes: inspection.Mounts.filter(item => item.Type === 'volume').map(item => item.Name) };
    writeFileSync(resolve(directory, 'engines.env'), `REDIS_URL=${env.REDIS_URL}\n`, { mode: 0o600 });
    writeFileSync(resolve(directory, 'manifest.json'), JSON.stringify(manifest, null, 2));
    const runtime = { directory, manifest, password, env };
    await readyRedis(runtime);
    manifest.redisVersion = redisCli(runtime, 'INFO', 'server').match(/redis_version:([^\r\n]+)/)?.[1];
    writeFileSync(resolve(directory, 'manifest.json'), JSON.stringify(manifest, null, 2));
    return runtime;
  } catch (error) {
    if (containerId) docker(['rm', '-fv', containerId]);
    rmSync(directory, { recursive: true, force: true });
    throw new Error(`Isolated Redis startup failed: ${error.status ?? error.code ?? 'unknown'}`);
  }
}
export function pauseRedis(runtime) { ownedRedis(runtime); docker(['stop', '-t', '2', runtime.manifest.containerId]); }
export async function resumeRedis(runtime) {
  ownedRedis(runtime); docker(['start', runtime.manifest.containerId]); await readyRedis(runtime);
  assert.equal(Number(ownedRedis(runtime).NetworkSettings.Ports['6379/tcp'][0].HostPort), runtime.manifest.port);
}
export async function restartRedis(runtime) { pauseRedis(runtime); await resumeRedis(runtime); }
export function stopRedis(runtime) {
  ownedRedis(runtime); docker(['rm', '-fv', runtime.manifest.containerId]);
  const absent = args => { try { docker(args); return false; } catch (error) { assert.equal(error.status, 1); return true; } };
  const cleanup = { containerAbsent: absent(['inspect', runtime.manifest.containerId]),
    volumesAbsent: runtime.manifest.volumes.every(name => absent(['volume', 'inspect', name])) };
  rmSync(runtime.directory, { recursive: true });
  cleanup.credentialsRemoved = (() => { try { readFileSync(resolve(runtime.directory, 'engines.env')); return false; } catch { return true; } })();
  assert.ok(Object.values(cleanup).every(Boolean)); return cleanup;
}
