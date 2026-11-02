import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { fileURLToPath, pathToFileURL } from 'node:url';

// External, opt-in driver: excluded from measured application source fingerprints.
// Preserve this exact hashed file for both sides; no proxy, reader, SSE or retry.
const driverPath = fileURLToPath(import.meta.url);
const defaultSource = fileURLToPath(new URL('../', import.meta.url));
const source = resolve(process.env.SPIKE_SOURCE || defaultSource);
const destination = resolve(process.env.EVIDENCE_DIR || resolve(defaultSource, 'evidence', `bulk-scaling-${Date.now()}`));
const moduleAt = path => import(pathToFileURL(resolve(source, path)).href);
const { startEngine, stopEngine } = await moduleAt('infra/runtime.mjs');
const { Gateways } = await moduleAt('infra/gateways.mjs');
const { provenance } = await moduleAt('infra/provenance.mjs');
const { fixture } = await moduleAt('harness/load.mjs');
const { request, batch, update, percentile } = await moduleAt('harness/client.mjs');
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
mkdirSync(destination); // Refuse to overwrite raw evidence.
const report = { status: 'running', startedAt: new Date().toISOString(), samples: [],
  configuration: { nodeCount: 5000, changedRows: [1, 3, 500], iterations: 12, writers: 1, readers: 0, sseClients: 0,
    proxy: false, retries: false, buildProfile: 'cargo dev (unoptimized + debuginfo)',
    wholeOperation: 'Lightweight revision GET, create the update payload, POST, read/parse response. No retry/backoff; seeding and final verification excluded.' },
  driverProvenance: { path: 'evidence/bulk-scaling-driver-1793608180.mjs', sha256: hash(driverPath) },
  clientProvenance: ['harness/client.mjs', 'harness/load.mjs'].map(path => ({ path, sha256: hash(resolve(source, path)) })) };
const save = () => writeFileSync(resolve(destination, 'bulk-scaling.json'), JSON.stringify(report, null, 2));
const quantiles = values => ({ p50: percentile(values, .5), p95: percentile(values, .95), p99: percentile(values, .99) });
let runtime, gateways;
try {
  report.provenance = provenance(); save();
  runtime = await startEngine(); report.topology = runtime.manifest;
  gateways = new Gateways(runtime, resolve(source, 'postgres/target/debug/oc-spike-postgres'));
  report.startup = await gateways.start(0);
  const url = gateways.urls[0];
  for (const changedRows of report.configuration.changedRows) {
    const { projectId, revision } = await fixture(url, report.configuration.nodeCount);
    const samples = [];
    for (let iteration = 0; iteration < report.configuration.iterations; iteration++) {
      const started = performance.now();
      const current = await request(url, `/projects/${projectId}/revision`);
      assert.equal(current.status, 200); assert.equal(current.body.revision, revision + iteration);
      const operations = Array.from({ length: changedRows }, (_, index) => update(`n-${index + 3}`, { name: String(iteration + 1) }));
      const result = await request(url, '/batch', batch(projectId, current.body.revision, operations));
      const wholeOperationMs = performance.now() - started;
      assert.equal(result.status, 200, JSON.stringify(result.body));
      assert.equal(result.body.changedIds.length, changedRows); assert.equal(result.body.committedRevision, revision + iteration + 1);
      samples.push({ iteration, changedRows, revisionGetMs: current.ms, postMs: result.ms, wholeOperationMs, committedRevision: result.body.committedRevision });
    }
    const final = (await request(url, `/projects/${projectId}`)).body;
    assert.equal(final.nodes.length, report.configuration.nodeCount); assert.equal(final.revision, revision + samples.length);
    for (let index = 0; index < changedRows; index++) assert.equal(final.nodes.find(node => node.id === `n-${index + 3}`).props.name, String(samples.length));
    const commits = (await request(url, `/changes/${projectId}?sinceRevision=${revision}`)).body.commits;
    assert.deepEqual(commits.map(value => value.revision), samples.map(value => value.committedRevision));
    writeFileSync(resolve(destination, `bulk-${changedRows}-samples.jsonl`), samples.map(value => JSON.stringify(value)).join('\n') + '\n', { flag: 'wx' });
    report.samples.push({ changedRows, projectId, successfulCount: samples.length, postMs: quantiles(samples.map(value => value.postMs)),
      wholeOperationMs: quantiles(samples.map(value => value.wholeOperationMs)), revisionGetMs: quantiles(samples.map(value => value.revisionGetMs)),
      operationsPerSecond: samples.length / (samples.reduce((total, value) => total + value.wholeOperationMs, 0) / 1000), observedCommits: commits.length });
    save(); console.log(JSON.stringify(report.samples.at(-1)));
  }
  report.provenanceAfter = provenance();
  assert.equal(report.provenance.sourceFingerprint, report.provenanceAfter.sourceFingerprint);
  assert.deepEqual(report.provenance.binaries, report.provenanceAfter.binaries);
  assert.equal(report.driverProvenance.sha256, hash(driverPath));
  report.status = 'pass';
} catch (error) { report.status = 'fail'; report.error = { message: error.message, stack: error.stack }; process.exitCode = 1; }
finally {
  try {
    if (gateways) await gateways.close(); if (runtime) stopEngine(runtime);
    report.cleanup = 'owned gateway and PostgreSQL container/anonymous volume removed';
  } catch (error) { report.cleanup = `failed: ${error.message}`; process.exitCode = 1; }
  report.finishedAt = new Date().toISOString(); save();
  console.log(JSON.stringify({ status: report.status, evidence: destination, error: report.error?.message, cleanup: report.cleanup }));
}
