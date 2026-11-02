import assert from 'node:assert/strict';
import { mkdirSync, rmSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath } from 'node:url';
import { docker, owned, sql, startEngine, stopEngine } from '../infra/runtime.mjs';
import { Gateways } from '../infra/gateways.mjs';
import { assertFrozen, freeze } from './single-writer-provenance.mjs';
import { singleWriterLoad } from './single-writer-load.mjs';
import { measuredProfiles, readProfiles } from './single-writer-profiles.mjs';

const root = fileURLToPath(new URL('../', import.meta.url));
const source = resolve(process.env.SPIKE_SOURCE || root);
const executable = resolve(process.env.SPIKE_BINARY || resolve(source, 'postgres/target/debug/oc-spike-postgres'));
const destination = resolve(process.env.EVIDENCE_DIR || resolve(root, 'evidence', `single-writer-${Date.now()}`));
const configuration = { nodeCounts: [100, 5000], writers: 1, samples: Number(process.env.SAMPLES || 100),
  warmup: Number(process.env.WARMUP || 20), buildProfile: 'cargo dev (unoptimized + debuginfo)',
  client: 'optimized commitFixture; revision GET + POST 3-node update; continuous reader + SSE',
  batchProfiling: true, buildDuringRun: false };
assert.ok(Number.isInteger(configuration.samples) && configuration.samples >= 100);
assert.ok(Number.isInteger(configuration.warmup) && configuration.warmup > 0);
mkdirSync(destination); // Raw evidence from an earlier run is never overwritten.
const report = { status: 'running', startedAt: new Date().toISOString(), configuration, cells: [],
  scope: 'One-writer diagnosis only; fresh disposable database, no application runtime/user data, no performance SLA or optimized comparison.' };
const save = () => writeFileSync(resolve(destination, 'report.json'), JSON.stringify(report, null, 2));
const jsonl = (name, records) => writeFileSync(resolve(destination, name), records.map(value => JSON.stringify(value)).join('\n') + '\n', { flag: 'wx' });
let runtime, gateways, volumeNames = [];
try {
  report.provenance = freeze(source, executable, root); save();
  runtime = await startEngine();
  report.topology = structuredClone(runtime.manifest);
  volumeNames = owned(runtime).Mounts.filter(value => value.Type === 'volume').map(value => value.Name);
  gateways = new Gateways({ ...runtime, env: { ...runtime.env, OC_SPIKE_PROFILE_BATCH: '1' } }, executable);
  report.startup = [await gateways.start(0), await gateways.start(1)]; save();
  assert.equal(Number(sql(runtime, 'SELECT count(*) FROM projects')), 0, 'Database must be fresh');
  for (const nodeCount of configuration.nodeCounts) {
    const before = freeze(source, executable, root);
    assertFrozen(report.provenance, before);
    const log = gateways.entries[0].logPath;
    const offset = readProfiles(log).length;
    console.log(JSON.stringify({ phase: 'single-writer', state: 'start', nodeCount, samples: configuration.samples }));
    const cell = await singleWriterLoad(gateways.urls, nodeCount, configuration.warmup, configuration.samples);
    const profiles = readProfiles(log).slice(offset);
    const profile = measuredProfiles(profiles, cell);
    jsonl(`${nodeCount}-attempts.jsonl`, cell.attempts);
    jsonl(`${nodeCount}-successes.jsonl`, cell.successes);
    jsonl(`${nodeCount}-reads.jsonl`, cell.reads);
    jsonl(`${nodeCount}-profiles-all.jsonl`, profiles);
    jsonl(`${nodeCount}-profiles-measured.jsonl`, profile.records);
    delete cell.attempts; delete cell.successes; delete cell.reads;
    cell.profile = profile.summary;
    cell.provenanceBefore = before;
    cell.provenanceAfter = freeze(source, executable, root);
    assertFrozen(before, cell.provenanceAfter);
    report.cells.push(cell); save();
    console.log(JSON.stringify({ phase: 'single-writer', state: 'pass', nodeCount, postMs: cell.postMs,
      wholeOperationMs: cell.wholeOperationMs, serverTotalMs: cell.profile.serverTotalMs }));
  }
  report.databaseCounts = Object.fromEntries(['projects', 'commits', 'idempotency'].map(table => [table, Number(sql(runtime, `SELECT count(*) FROM ${table}`))]));
  const expectedCommits = report.cells.reduce((total, cell) => total + cell.seedCommits + cell.warmup + cell.samples, 0);
  assert.deepEqual(report.databaseCounts, { projects: 2, commits: expectedCommits, idempotency: expectedCommits });
  report.provenanceAfter = freeze(source, executable, root);
  assertFrozen(report.provenance, report.provenanceAfter);
  report.status = 'pass';
} catch (error) {
  report.status = 'fail'; report.error = { message: error.message, stack: error.stack }; process.exitCode = 1;
} finally {
  try {
    if (gateways) await gateways.close();
    if (runtime) {
      report.gatewayProfiles = gateways.entries.map(entry => ({ pid: entry.pid, port: entry.port,
        records: readProfiles(entry.logPath).length }));
      stopEngine(runtime);
      const absent = args => {
        try { docker(args); return false; } catch (error) { assert.equal(error.status, 1); return true; }
      };
      const processesAbsent = gateways.entries.every(({ pid }) => {
        try { process.kill(pid, 0); return false; } catch (error) { assert.equal(error.code, 'ESRCH'); return true; }
      });
      report.cleanup = { containerAbsent: absent(['inspect', runtime.manifest.containerId]),
        volumesAbsent: volumeNames.every(name => absent(['volume', 'inspect', name])), processesAbsent,
        credentialsRemoved: true };
      rmSync(runtime.directory, { recursive: true });
      assert.ok(Object.values(report.cleanup).every(Boolean), 'Owned resources must be removed');
    }
  } catch (error) { report.cleanupError = error.message; report.status = 'fail'; process.exitCode = 1; }
  report.finishedAt = new Date().toISOString(); save();
  console.log(JSON.stringify({ status: report.status, evidence: destination, error: report.error?.message, cleanup: report.cleanup }));
}
