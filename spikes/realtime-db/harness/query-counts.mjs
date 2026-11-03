import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { mkdirSync, readFileSync, writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';
import { wireProbe } from '../infra/pg-wire-probe.mjs';

// Opt-in diagnostic only: the transparent observer is never in matrix timings.
const ownRoot = fileURLToPath(new URL('../', import.meta.url));
const source = resolve(process.env.SPIKE_SOURCE || ownRoot);
const destination = resolve(process.env.EVIDENCE_DIR || resolve(ownRoot, 'evidence', `query-counts-${Date.now()}`));
const moduleAt = path => import(pathToFileURL(resolve(source, path)).href);
const { startEngine, stopEngine } = await moduleAt('infra/runtime.mjs');
const { Gateways } = await moduleAt('infra/gateways.mjs');
const { provenance } = await moduleAt('infra/provenance.mjs');
const { request, batch, create, update, pause } = await moduleAt('harness/client.mjs');
mkdirSync(destination); // Preserve previous evidence; never overwrite a run.
const hash = path => createHash('sha256').update(readFileSync(path)).digest('hex');
const proofFiles = ['harness/query-counts.mjs', 'infra/pg-wire-probe.mjs'];
const report = { status: 'running', startedAt: new Date().toISOString(), samples: [],
  method: 'Plaintext PostgreSQL v3 loopback framing; one idle gateway, no reader/SSE. Aggregate tags/categories only; no auth, Bind, row payloads or raw SQL persisted.',
  definitions: { sqlExecutions: 'Frontend Execute + SimpleQuery; includes BEGIN/COMMIT, excludes Parse/Bind/Close.',
    synchronizationCycles: 'Frontend Sync + SimpleQuery matched against backend ReadyForQuery. Observed protocol completion barriers, not TCP packets or latency measurements.',
    statements: 'Categories of Parse/SimpleQuery SQL templates, without SQL text/parameter values.' },
  proofProvenance: proofFiles.map(path => ({ path, sha256: hash(resolve(ownRoot, path)) })) };
let runtime, gateways, probe;
const save = () => writeFileSync(resolve(destination, 'query-counts.json'), JSON.stringify(report, null, 2));
try {
  report.provenance = provenance(); save();
  runtime = await startEngine(); report.topology = runtime.manifest;
  probe = await wireProbe(runtime.manifest.port);
  const proxyUrl = new URL(runtime.env.PG_URL); proxyUrl.port = String(probe.port);
  gateways = new Gateways({ ...runtime, env: { ...runtime.env, PG_URL: proxyUrl.href } }, resolve(source, 'postgres/target/debug/oc-spike-postgres'));
  await gateways.start(0);
  const url = gateways.urls[0];
  const cases = [1, 3, 500].flatMap(changedRows => ['update', 'delete'].map(kind => ({ kind, changedRows })));
  cases.push({ kind: 'mixed', changedRows: 500 });
  for (const { kind, changedRows } of cases) {
    const projectId = `wire-${kind}-${changedRows}-${Date.now()}`;
    assert.equal((await request(url, '/projects', { projectId })).status, 200);
    const nodes = [create('root'), ...Array.from({ length: 500 }, (_, n) => create(`n-${n}`, 'root', 'text', { text: `Node ${n}` }))];
    let revision = 0;
    for (let n = 0; n < nodes.length; n += 500) {
      const result = await request(url, '/batch', batch(projectId, revision, nodes.slice(n, n + 500)));
      assert.equal(result.status, 200, JSON.stringify(result.body)); revision = result.body.committedRevision;
    }
    const operations = Array.from({ length: changedRows }, (_, n) => kind === 'delete' || (kind === 'mixed' && n % 2)
      ? { op: 'delete_subtree', id: `n-${n}` } : update(`n-${n}`, { name: 'changed' }));
    await pause(20); probe.checkpoint();
    const result = await request(url, '/batch', batch(projectId, revision, operations));
    assert.equal(result.status, 200, JSON.stringify(result.body));
    assert.equal(result.body.changedIds.length, changedRows);
    assert.equal(result.body.committedRevision, revision + 1);
    await pause(20);
    const protocol = probe.result();
    assert.equal(protocol.synchronizationCycles, protocol.readyForQuery, 'Every measured protocol barrier must complete');
    assert.equal(protocol.statements.begin, 1); assert.equal(protocol.statements.commit, 1);
    assert.equal(protocol.sqlExecutions, Object.values(protocol.statements).reduce((a, b) => a + b, 0), 'No prepared SQL execution is missing or duplicated');
    report.samples.push({ kind, changedRows, protocol }); save();
    console.log(JSON.stringify({ kind, changedRows, sqlExecutions: protocol.sqlExecutions, synchronizationCycles: protocol.synchronizationCycles }));
  }
  report.provenanceAfter = provenance();
  assert.equal(report.provenance.sourceFingerprint, report.provenanceAfter.sourceFingerprint, 'Measured source must stay frozen');
  assert.deepEqual(report.proofProvenance, proofFiles.map(path => ({ path, sha256: hash(resolve(ownRoot, path)) })), 'Proof runner must stay frozen');
  report.status = 'pass';
} catch (error) { report.status = 'fail'; report.error = { message: error.message, stack: error.stack }; process.exitCode = 1; }
finally {
  try {
    if (gateways) await gateways.close(); if (probe) await probe.close(); if (runtime) stopEngine(runtime);
    report.cleanup = 'owned gateway, proxy sockets and PostgreSQL container/anonymous volume removed';
  } catch (error) { report.cleanup = `failed: ${error.message}`; process.exitCode = 1; }
  report.finishedAt = new Date().toISOString(); save();
  console.log(JSON.stringify({ status: report.status, evidence: destination, error: report.error?.message, cleanup: report.cleanup }));
}
