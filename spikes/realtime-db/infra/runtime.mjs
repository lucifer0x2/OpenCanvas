import { execFile, execFileSync } from 'node:child_process';
import { promisify } from 'node:util';
import { createServer } from 'node:net';
import { arch, cpus, platform, totalmem } from 'node:os';
import { randomBytes } from 'node:crypto';
import { chmodSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { pause } from '../harness/client.mjs';

export const root = fileURLToPath(new URL('../', import.meta.url));
export const image = 'postgres:16.9@sha256:ddfe3e8713e3ee5b8f286082cb12512488dfbf3f5a1ecb0b74a42e6055af0a5f';
export function docker(args, options = {}) {
  return execFileSync(process.env.DOCKER || '/usr/local/bin/docker', args,
    { encoding: 'utf8', stdio: ['pipe', 'pipe', 'pipe'], ...options });
}
const execute = promisify(execFile);
async function dockerAsync(args) {
  return execute(process.env.DOCKER || '/usr/local/bin/docker', args, { encoding: 'utf8', timeout: 30000 });
}
export function readRuntime(directory) {
  const manifest = JSON.parse(readFileSync(resolve(directory, 'manifest.json')));
  const env = Object.fromEntries(readFileSync(resolve(directory, 'engines.env'), 'utf8').trim().split('\n').map(line => {
    const index = line.indexOf('='); return [line.slice(0, index), line.slice(index + 1)];
  }));
  return { directory, manifest, env };
}
export function owned(runtime) {
  const value = JSON.parse(docker(['inspect', runtime.manifest.containerId]))[0];
  if (value.Config.Labels['opencanvas.spike'] !== runtime.manifest.runId) throw new Error('Container ownership mismatch');
  return value;
}
export async function ready(runtime) {
  for (let n = 0; n < 100; n++) {
    try { sql(runtime, 'SELECT 1'); return; } catch { await pause(100); }
  }
  throw new Error('PostgreSQL readiness timeout');
}
export async function startEngine() {
  const runId = `pg-${Date.now()}-${randomBytes(4).toString('hex')}`;
  const directory = resolve(root, '.runtime', runId);
  mkdirSync(directory, { recursive: true, mode: 0o700 });
  chmodSync(directory, 0o700);
  const credential = resolve(directory, 'postgres.env');
  writeFileSync(credential, `POSTGRES_PASSWORD=${randomBytes(24).toString('hex')}\nPOSTGRES_DB=spike\n`, { mode: 0o600 });
  let containerId;
  try {
    const socket = createServer();
    await new Promise((resolve, reject) => { socket.once('error', reject); socket.listen(0, '127.0.0.1', resolve); });
    const selectedPort = socket.address().port;
    await new Promise(resolve => socket.close(resolve));
    // Docker atomically claims this random port; on a race startup fails safely.
    // Explicit mapping stays stable across stop/start, unlike an empty HostPort.
    containerId = docker(['run', '-d', '--name', runId, '--label', `opencanvas.spike=${runId}`,
      '--cpus', '2', '--memory', '2g', '--memory-swap', '2g', '-p', `127.0.0.1:${selectedPort}:5432`,
      '--env-file', credential, image]).trim();
    const inspected = JSON.parse(docker(['inspect', containerId]))[0];
    const port = Number(inspected.NetworkSettings.Ports['5432/tcp'][0].HostPort);
    const password = readFileSync(credential, 'utf8').split('\n')[0].split('=')[1];
    const env = { PG_URL: `postgresql://postgres:${password}@127.0.0.1:${port}/spike` };
    writeFileSync(resolve(directory, 'engines.env'), `PG_URL=${env.PG_URL}\n`, { mode: 0o600 });
    const manifest = { runId, createdAt: new Date().toISOString(), containerId, image,
      imageId: inspected.Image, port, cpu: 2, memoryBytes: 2147483648,
      dockerVersion: JSON.parse(docker(['version', '--format', '{{json .}}'])),
      context: docker(['context', 'show']).trim(), node: process.version,
      gatewayHost: { platform: platform(), arch: arch(), cpuModel: cpus()[0]?.model, logicalCpus: cpus().length, memoryBytes: totalmem(), limits: 'uncapped host processes' },
      topology: 'One PostgreSQL container; two independent host gateway processes; loopback only; no Kubernetes',
      inheritedRuntimePolicy: 'Never read or stop resources from earlier .runtime/engines.json' };
    writeFileSync(resolve(directory, 'manifest.json'), JSON.stringify(manifest, null, 2));
    const runtime = { directory, manifest, env };
    await ready(runtime);
    manifest.postgresVersion = sql(runtime, 'SELECT version()').trim();
    writeFileSync(resolve(directory, 'manifest.json'), JSON.stringify(manifest, null, 2));
    return runtime;
  } catch (error) {
    if (containerId) docker(['rm', '-fv', containerId]);
    throw new Error(`Isolated PostgreSQL startup failed: ${error.status ?? error.code ?? 'unknown'}`);
  }
}
export function sql(runtime, query, database = 'spike') {
  return docker(['exec', '-i', runtime.manifest.containerId, 'psql', '-X', '-v', 'ON_ERROR_STOP=1', '-U', 'postgres', '-d', database, '-At'], { input: query });
}
export function stopEngine(runtime) {
  owned(runtime); docker(['rm', '-fv', runtime.manifest.containerId]);
  runtime.manifest.cleanedAt = new Date().toISOString();
  writeFileSync(resolve(runtime.directory, 'manifest.json'), JSON.stringify(runtime.manifest, null, 2));
}
export async function restartEngine(runtime, downtimeMs = 300) {
  owned(runtime); await dockerAsync(['stop', '-t', '5', runtime.manifest.containerId]);
  await pause(downtimeMs); await dockerAsync(['start', runtime.manifest.containerId]);
  const inspection = await dockerAsync(['inspect', runtime.manifest.containerId]);
  const port = Number(JSON.parse(inspection.stdout)[0].NetworkSettings.Ports['5432/tcp'][0].HostPort);
  if (port !== runtime.manifest.port) throw new Error('Published database port changed during restart');
  for (let n = 0; n < 100; n++) {
    try {
      await dockerAsync(['exec', runtime.manifest.containerId, 'psql', '-X', '-U', 'postgres', '-d', 'spike', '-Atc', 'SELECT 1']);
      return;
    } catch { await pause(100); }
  }
  throw new Error('PostgreSQL restart readiness timeout');
}
export function databaseStats(runtime) {
  return { docker: JSON.parse(docker(['stats', '--no-stream', '--format', '{{json .}}', runtime.manifest.containerId])),
    databaseBytes: Number(sql(runtime, "SELECT pg_database_size('spike')")),
    commitRows: Number(sql(runtime, 'SELECT count(*) FROM commits')),
    idempotencyRows: Number(sql(runtime, 'SELECT count(*) FROM idempotency')),
    rssBytes: Number(docker(['exec', runtime.manifest.containerId, 'sh', '-c',
      "awk '/VmRSS/ {sum += $2} END {print sum * 1024}' /proc/[0-9]*/status"]).trim()),
    rssDefinition: 'Sum process VmRSS; shared pages may be counted multiple times; docker memory usage recorded separately' };
}
