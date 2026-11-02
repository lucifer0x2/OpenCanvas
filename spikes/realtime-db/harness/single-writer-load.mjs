import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';
import { percentile, request, subscribe, update } from './client.mjs';
import { fixture } from './load.mjs';
import { commitFixture } from './retry.mjs';

export function quantiles(values) {
  assert.ok(values.length > 0, 'Latency samples must be nonempty');
  assert.ok(values.every(value => Number.isFinite(value) && value >= 0));
  return { count: values.length, p50: percentile(values, .5), p95: percentile(values, .95),
    p99: percentile(values, .99), min: Math.min(...values), max: Math.max(...values) };
}

export async function singleWriterLoad(urls, nodeCount, warmup, samples) {
  assert.equal(urls.length, 2);
  assert.ok(nodeCount >= 4 && warmup > 0 && samples >= 100);
  const { projectId, revision } = await fixture(urls[0], nodeCount);
  const stream = subscribe(urls[1], projectId, revision);
  await stream.ready;
  const attempts = [], successes = [], reads = [];
  let reading = true, readerError, latestRequest;
  const reader = (async () => {
    try {
      while (reading) {
        const result = await request(urls[reads.length % 2], `/projects/${projectId}`);
        assert.equal(result.status, 200);
        const a = result.body.nodes.find(node => node.id === 'sentinel-a');
        const b = result.body.nodes.find(node => node.id === 'sentinel-b');
        assert.ok(a && b); assert.equal(a.props.name, b.props.name);
        assert.equal(result.body.nodes.length, nodeCount);
        reads.push({ revision: result.body.revision, ms: result.ms });
      }
    } catch (error) { readerError = error; }
  })();
  let measuredStart, measuredEnd, measuredReadStart, measuredReadEnd;
  try {
    for (let iteration = 0; iteration < warmup + samples; iteration++) {
      if (readerError) throw readerError;
      if (iteration === warmup) { measuredStart = performance.now(); measuredReadStart = reads.length; }
      const start = performance.now(), measured = iteration >= warmup;
      const name = `0-${iteration}`;
      const operations = [update('sentinel-a', { name }), update('sentinel-b', { name }),
        update('n-3', { name: String(iteration + 1) })];
      const send = async (url, path, body) => {
        if (path === '/batch') latestRequest = body;
        return request(url, path, body);
      };
      const result = await commitFixture({ url: urls[0], projectId, operations, send,
        onAttempt: attempt => attempts.push({ iteration, measured, ...attempt }) });
      assert.equal(result.revision, revision + iteration + 1);
      successes.push({ iteration, measured, ...result, ms: performance.now() - start });
    }
    measuredEnd = performance.now();
    measuredReadEnd = reads.length;
    assert.ok(measuredReadEnd > measuredReadStart, 'Snapshot reader must run during measured edits');
    const expected = Array.from({ length: warmup + samples }, (_, n) => revision + n + 1);
    const events = await stream.until(expected.length, 15000);
    assert.deepEqual(events.map(event => event.revision), expected);
    const final = await request(urls[0], `/projects/${projectId}`);
    assert.equal(final.status, 200);
    assert.equal(final.body.revision, expected.at(-1));
    assert.equal(final.body.nodes.length, nodeCount);
    for (const node of final.body.nodes.filter(node => node.id.startsWith('n-'))) {
      const n = Number(node.id.slice(2));
      const props = { text: `Node ${n}`, x: n % 100, y: Math.floor(n / 100) };
      if (n === 3) props.name = String(warmup + samples);
      assert.deepEqual(node.props, props);
    }
    const sentinelNames = final.body.nodes.filter(node => node.id.startsWith('sentinel-')).map(node => node.props.name);
    assert.deepEqual(sentinelNames, [`0-${warmup + samples - 1}`, `0-${warmup + samples - 1}`]);
    const changes = await request(urls[1], `/changes/${projectId}?sinceRevision=${revision}`);
    assert.equal(changes.status, 200);
    assert.deepEqual(changes.body.commits.map(commit => commit.revision), expected);
    const replay = await request(urls[1], '/batch', latestRequest);
    assert.equal(replay.status, 200); assert.equal(replay.body.replayed, true);
    assert.equal(replay.body.committedRevision, expected.at(-1));
    const afterReplay = await request(urls[1], `/projects/${projectId}`);
    assert.deepEqual(afterReplay.body, final.body);
    assert.ok(reads.length > 0);
    const measuredAttempts = attempts.filter(value => value.measured);
    const measuredSuccesses = successes.filter(value => value.measured);
    assert.equal(measuredSuccesses.length, samples);
    assert.equal(attempts.length, warmup + samples, 'One writer has no expected conflicts/retries');
    assert.ok(attempts.every(value => value.status === 200));
    return { nodeCount, writers: 1, warmup, samples, projectId, seedCommits: Math.ceil(nodeCount / 500),
      successfulCount: successes.length, observedCommits: changes.body.commits.length, observedEvents: events.length,
      readCount: reads.length, measuredReadCount: measuredReadEnd - measuredReadStart,
      mixedSnapshots: 0, unchangedNodesVerified: nodeCount - 3, replayVerified: true,
      measuredDurationMs: measuredEnd - measuredStart, throughput: samples / ((measuredEnd - measuredStart) / 1000),
      postMs: quantiles(measuredAttempts.map(value => value.ms)),
      wholeOperationMs: quantiles(measuredSuccesses.map(value => value.ms)),
      revisionReadMs: quantiles(measuredSuccesses.map(value => value.revisionReadMs)),
      attempts, successes, reads };
  } finally {
    reading = false; await reader; await stream.close();
    if (readerError) throw readerError;
  }
}
