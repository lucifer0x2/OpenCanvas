import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';
import { batch, pause, percentile, request, update } from './client.mjs';
import { fixture } from './load.mjs';
import { reconnectingStream } from './reconnect.mjs';

export async function recovery(urls, disruption, label) {
  const { projectId, revision } = await fixture(urls[0], 100);
  const stream = reconnectingStream(urls[0], projectId, revision);
  const raw = [], successful = [], snapshots = [];
  const trafficStart = performance.now();
  const deadline = trafficStart + 30000;
  let stop = false;
  const safe = async (url, path, payload) => {
    const start = performance.now();
    try { const result = await request(url, path, payload, {}, 2000); raw.push({ atMs: start - trafficStart, path, status: result.status, ms: result.ms, error: result.body.error?.code }); return result; }
    catch (error) { raw.push({ atMs: start - trafficStart, path, status: 0, ms: performance.now() - start, error: error.message }); return null; }
  };
  const writer = (async () => {
    let iteration = 0;
    while (!stop || iteration < 20) {
      if (performance.now() > deadline) throw new Error('Recovery total traffic deadline exceeded (30s)');
      const start = performance.now();
      const current = await safe(urls[iteration % 2], `/projects/${projectId}`);
      if (current?.status !== 200) { await pause(10); continue; }
      const name = `recovery-${iteration}`;
      let payload = batch(projectId, current.body.revision, [update('sentinel-a', { name }), update('sentinel-b', { name })]);
      for (let retry = 0; ; retry++) {
        if (performance.now() > deadline) throw new Error('Recovery retry deadline exceeded (30s)');
        if (retry > 300) throw new Error('Recovery retry exhausted');
        const result = await safe(urls[(iteration + retry) % 2], '/batch', payload);
        if (result?.status === 200) { successful.push({ atMs: performance.now() - trafficStart, iteration, revision: result.body.committedRevision, retries: retry, ms: performance.now() - start }); break; }
        if (result?.status === 409 && result.body.error?.code === 'REVISION_CONFLICT') {
          const state = await safe(urls[1], `/projects/${projectId}`);
          if (state?.status === 200) payload = batch(projectId, state.body.revision, payload.operations);
        } else if (result && result.status < 500) throw new Error(JSON.stringify(result));
        await pause(10);
      }
      iteration++; await pause(5);
    }
  })();
  writer.catch(() => {}); // Await below; prevent early unhandled rejection during disruption.
  const reader = (async () => {
    let index = 0;
    while (!stop) {
      const result = await safe(urls[index++ % 2], `/projects/${projectId}`);
      if (result?.status === 200) {
        const values = result.body.nodes.filter(node => node.id.startsWith('sentinel-')).map(node => node.props.name);
        snapshots.push({ revision: result.body.revision, matching: values.length === 2 && values[0] === values[1] });
      }
      await pause(10);
    }
  })();
  reader.catch(() => {});
  let disruptionResult, disruptionStart, disruptionEnd;
  try {
  try { await pause(150); disruptionStart = performance.now() - trafficStart; disruptionResult = await disruption(); disruptionEnd = performance.now() - trafficStart; await pause(300); }
  finally { stop = true; await Promise.all([writer, reader]); }
  for (let n = 0; n < 500 && stream.events.length < successful.length; n++) await pause(20);
  const final = (await request(urls[1], `/projects/${projectId}`)).body;
  const expected = Array.from({ length: successful.length }, (_, index) => revision + index + 1);
  assert.deepEqual(successful.map(value => value.revision), expected);
  assert.deepEqual(stream.events.map(value => value.revision), expected);
  assert.equal(final.revision, revision + successful.length);
  assert.ok(snapshots.length > 0); assert.ok(snapshots.every(value => value.matching));
  assert.ok(successful.length >= 20);
  const overlappingRequests = raw.filter(value => value.atMs >= disruptionStart && value.atMs <= disruptionEnd).length;
  assert.ok(overlappingRequests > 0, 'Traffic must overlap the disruption');
  return { label, projectId, disruptionResult, successfulCount: successful.length,
    disruptionWindowMs: [disruptionStart, disruptionEnd], overlappingRequests,
    readCount: snapshots.length, mixedSnapshots: snapshots.filter(value => !value.matching).length,
    rawErrorCount: raw.filter(value => value.status !== 200).length,
    rawErrors: raw.filter(value => value.status !== 200), raw, successful,
    retryToSuccessMs: { p95: percentile(successful.map(value => value.ms), .95), p99: percentile(successful.map(value => value.ms), .99) },
    sse: { attempts: stream.attempts, reconnectGapsMs: stream.gapsMs, errors: stream.errors,
      duplicateCommits: stream.events.length - new Set(stream.events.map(value => value.revision)).size,
      missingCommits: expected.filter(revision => !stream.events.some(value => value.revision === revision)), observed: stream.events.length },
    finalRevision: final.revision };
  } catch (error) {
    error.evidence = { raw, successful, sseEvents: stream.events, sseErrors: stream.errors };
    throw error;
  } finally { stop = true; await Promise.allSettled([writer, reader]); await stream.close(); }
}
