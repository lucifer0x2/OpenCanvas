import { execFileSync } from 'node:child_process';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { startEngine, stopEngine, restartEngine, root } from '../infra/runtime.mjs';
import { startRedis, stopRedis } from '../infra/redis-runtime.mjs';
import { Gateways } from '../infra/gateways.mjs';
import { contractMatrix } from './run.mjs';
import { recovery } from './recovery.mjs';
import { history } from './history.mjs';
import { migration } from './migration.mjs';
import { provenance } from '../infra/provenance.mjs';

// All lifecycle belongs to this run; no inherited manifest/process is touched.
const report = { startedAt: new Date().toISOString(), status: 'running', phases: {},
  untested: ['real MCP sessions', 'actual rendering', 'Kubernetes', 'HA/failover', 'DB major upgrade/mixed version', 'SQLite cutover', 'external object store'] };
let runtime, redis, gateways, output;
const selected = process.env.PHASES?.split(',').filter(Boolean);
report.validationScope = selected || 'full default phase sequence';
report.configuration = { nodeCounts: process.env.NODE_COUNTS || '100,5000', writers: process.env.WRITERS || '1,4,8',
  iterations: Number(process.env.ITERATIONS || 12), historyCommits: Number(process.env.HISTORY_COMMITS || 1000),
  clientModes: process.env.CLIENT_MODES || 'legacy,optimized', buildProfile: 'cargo dev (unoptimized + debuginfo)',
  contractTests: process.env.CONTRACT_TESTS !== '0', skipBuild: process.env.SKIP_BUILD === '1' };
const save = () => { if (output) writeFileSync(resolve(output, 'report.json'), JSON.stringify(report, null, 2)); };
const phase = async (name, action) => {
  if (name !== 'startup' && selected && !selected.includes(name)) { report.phases[name] = { status: 'skipped' }; save(); return; }
  console.log(JSON.stringify({ phase: name, state: 'start' }));
  const start = performance.now();
  report.phases[name] = { status: 'running' }; save();
  try {
    const result = await action();
    report.phases[name] = { status: 'pass', durationMs: performance.now() - start, result }; save();
  } catch (error) {
    report.phases[name] = { status: 'fail', durationMs: performance.now() - start, error: error.message, evidence: error.evidence }; save(); throw error;
  }
};
try {
  if (process.env.SKIP_BUILD !== '1') {
    execFileSync('cargo', ['build', '--offline', '--locked', '--manifest-path', resolve(root, 'postgres/Cargo.toml')], { stdio: 'inherit' });
    execFileSync('cargo', ['build', '--offline', '--locked', '--manifest-path', resolve(root, 'postgres/Cargo.toml'), '--features', 'compat-v2', '--target-dir', resolve(root, '.runtime/v2-target')], { stdio: 'inherit' });
  }
  runtime = await startEngine();
  if (report.configuration.contractTests && (!selected || selected.includes('rustContract'))) redis = await startRedis();
  mkdirSync(resolve(root, 'evidence'), { recursive: true });
  const destination = resolve(process.env.EVIDENCE_DIR || resolve(root, 'evidence', runtime.manifest.runId));
  mkdirSync(destination); // Refuse to overwrite any previous raw evidence.
  output = destination;
  report.runId = runtime.manifest.runId; report.topology = runtime.manifest;
  if (redis) report.redisTopology = structuredClone(redis.manifest);
  report.provenance = provenance();
  writeFileSync(resolve(output, 'provenance.json'), JSON.stringify(report.provenance, null, 2));
  writeFileSync(resolve(output, 'topology.json'), JSON.stringify(runtime.manifest, null, 2));
  console.log(JSON.stringify({ runtime: runtime.directory, evidence: output }));
  gateways = new Gateways({ ...runtime, env: { ...runtime.env, REDIS_URL: '' } }, resolve(root, 'postgres/target/debug/oc-spike-postgres'));
  await phase('startup', async () => [await gateways.start(0), await gateways.start(1)]);
  if (process.env.CONTRACT_TESTS !== '0') await phase('rustContract', async () => {
    const result = execFileSync('cargo', ['test', '--offline', '--manifest-path', resolve(root, 'postgres/Cargo.toml'), '--test', 'contract', '--', '--test-threads=1'],
      { env: { ...process.env, ...runtime.env, ...redis.env }, encoding: 'utf8', maxBuffer: 4 * 1024 * 1024 });
    writeFileSync(resolve(output, 'rust-contract.txt'), result); return { log: 'rust-contract.txt' };
  });
  await phase('contractAndMatrix', () => contractMatrix(gateways.urls, output));
  await phase('durableJobsResourcesStaging', async () => {
    const { exercise } = await import('../postgres/tests/durability.mjs');
    return exercise(gateways.urls, { killGateway: index => gateways.kill(index), restartGateway: index => gateways.restart(index, 0) });
  });
  await phase('rollingGateway', () => recovery(gateways.urls, async () => {
    const first = await gateways.restart(0); const second = await gateways.restart(1); return { first, second };
  }, 'Two gateway process kills/restarts one at a time; no Kubernetes rollout'));
  await phase('singleDatabaseOutage', () => recovery(gateways.urls, async () => {
    const start = performance.now(); await restartEngine(runtime, 700); return { outageAndRecoveryMs: performance.now() - start };
  }, 'Stop/start sole database host; expected availability outage; not HA or rolling host upgrade'));
  await phase('historyGrowth', () => history(runtime, gateways, Number(process.env.HISTORY_COMMITS || 1000)));
  await phase('additiveMigrationAndBackupRestore', () => migration(runtime, gateways));
  await phase('indexEvidence', async () => {
    const { indexEvidence } = await import('./indexes.mjs');
    return indexEvidence(runtime, output);
  });
  report.provenanceAfter = provenance();
  if (report.provenance.sourceFingerprint !== report.provenanceAfter.sourceFingerprint) throw new Error('Source changed during run; rerun after source freeze');
  report.status = 'pass';
} catch (error) {
  report.status = 'fail'; report.error = { message: error.message, stack: error.stack };
  process.exitCode = 1;
} finally {
  try {
    if (gateways) await gateways.close();
    if (runtime) { stopEngine(runtime); rmSync(runtime.directory, { recursive: true }); }
    if (redis) report.redisCleanup = stopRedis(redis);
    report.cleanup = 'owned gateway processes, PostgreSQL/Redis containers/anonymous volumes and private credentials removed';
  }
  catch (error) { report.cleanup = `failed: ${error.message}`; process.exitCode = 1; }
  report.finishedAt = new Date().toISOString(); save();
  if (output) writeFileSync(resolve(root, 'evidence/latest.json'), JSON.stringify({ runId: report.runId, directory: output, status: report.status }, null, 2));
  console.log(JSON.stringify({ status: report.status, evidence: output, error: report.error?.message, cleanup: report.cleanup }));
}
