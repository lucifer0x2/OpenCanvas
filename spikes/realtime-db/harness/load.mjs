import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';
import { batch, create, key, percentile, request, subscribe, update } from './client.mjs';
import { commitFixture, retryPolicy } from './retry.mjs';

export async function fixture(url, nodeCount) {
  const projectId = `load-${nodeCount}-${key()}`;
  assert.equal((await request(url, '/projects', { projectId })).status, 200);
  const operations = [create('root'), create('sentinel-a', 'root', 'text', { name: '0' }), create('sentinel-b', 'root', 'text', { name: '0' })];
  for (let n = 3; n < nodeCount; n++) operations.push(create(`n-${n}`, 'root', 'text', { text: `Node ${n}`, x: n % 100, y: Math.floor(n / 100) }));
  let revision = 0;
  for (let n = 0; n < operations.length; n += 500) {
    const result = await request(url, '/batch', batch(projectId, revision, operations.slice(n, n + 500)));
    assert.equal(result.status, 200, JSON.stringify(result.body));
    revision = result.body.committedRevision;
  }
  assert.equal((await request(url, `/projects/${projectId}`)).body.nodes.length, nodeCount);
  return { projectId, revision };
}

export async function load(urls, nodeCount, writers, iterations = 12, clientMode = 'optimized') {
  assert.ok(['legacy', 'optimized'].includes(clientMode), `Unknown client mode: ${clientMode}`);
  assert.ok(nodeCount >= writers + 3 && writers > 0 && iterations > 0, 'Require real nonempty fixture traffic');
  const { projectId, revision } = await fixture(urls[0], nodeCount);
  const attempts = [];
  const successes = [];
  const readerErrors = [];
  let reading = true;
  let readCount = 0;
  const stream = subscribe(urls[1], projectId, revision);
  await stream.ready;
  const reader = (async () => {
    while (reading) {
      const result = await request(urls[readCount % urls.length], `/projects/${projectId}`);
      if (result.status !== 200) throw new Error(`Snapshot HTTP ${result.status}`);
      const a = result.body.nodes.find(node => node.id === 'sentinel-a');
      const b = result.body.nodes.find(node => node.id === 'sentinel-b');
      if (!a || !b || a.props.name !== b.props.name) readerErrors.push({ revision: result.body.revision, a, b });
      readCount++;
    }
  })();
  const start = performance.now();
  try {
    await Promise.all(Array.from({ length: writers }, async (_, writer) => {
      const url = urls[writer % urls.length];
      for (let iteration = 0; iteration < iterations; iteration++) {
        const operationStart = performance.now();
        if (clientMode === 'optimized') {
          const name = `${writer}-${iteration}`;
          const result = await commitFixture({ url, projectId,
            operations: [update('sentinel-a', { name }), update('sentinel-b', { name }), update(`n-${writer + 3}`, { name: String(iteration + 1) })],
            onAttempt: attempt => attempts.push({ writer, iteration, ...attempt }) });
          successes.push({ writer, iteration, ...result, ms: performance.now() - operationStart });
          continue;
        }
        let retry = 0;
        while (true) {
          if (retry > 1000) throw new Error('Conflict retry bound exceeded');
          const current = await request(url, `/projects/${projectId}`);
          assert.equal(current.status, 200);
          const name = `${writer}-${iteration}`;
          const payload = batch(projectId, current.body.revision, [update('sentinel-a', { name }), update('sentinel-b', { name }), update(`n-${writer + 3}`, { name: String(iteration + 1) })]);
          const result = await request(url, '/batch', payload);
          attempts.push({ writer, iteration, retry, key: payload.idempotencyKey, expectedRevision: payload.expectedRevision, status: result.status, ms: result.ms, error: result.body.error?.code });
          if (result.status === 409 && result.body.error?.code === 'REVISION_CONFLICT') { retry++; continue; }
          assert.equal(result.status, 200, JSON.stringify(result.body));
          successes.push({ writer, iteration, retries: retry, ms: performance.now() - operationStart, revision: result.body.committedRevision });
          break;
        }
      }
    }));
    assert.equal(successes.length, writers * iterations);
    const events = await stream.until(successes.length, 15000);
    const expected = Array.from({ length: successes.length }, (_, i) => revision + i + 1);
    assert.deepEqual(events.map(event => event.revision), expected);
    const final = (await request(urls[0], `/projects/${projectId}`)).body;
    assert.equal(final.revision, revision + successes.length);
    assert.equal(final.nodes.length, nodeCount);
    for (let writer = 0; writer < writers; writer++) assert.equal(final.nodes.find(node => node.id === `n-${writer + 3}`).props.name, String(iterations));
    for (const node of final.nodes.filter(node => node.id.startsWith('n-'))) {
      const n = Number(node.id.slice(2));
      const expectedProps = { text: `Node ${n}`, x: n % 100, y: Math.floor(n / 100) };
      if (n < writers + 3) expectedProps.name = String(iterations);
      assert.deepEqual(node.props, expectedProps);
    }
    const names = final.nodes.filter(node => node.id.startsWith('sentinel-')).map(node => node.props.name);
    assert.equal(names[0], names[1]);
    const [lastWriter, lastIteration] = names[0].split('-').map(Number);
    assert.ok(lastWriter >= 0 && lastWriter < writers && lastIteration === iterations - 1);
    const commits = (await request(urls[1], `/changes/${projectId}?sinceRevision=${revision}`)).body.commits;
    assert.deepEqual(commits.map(commit => commit.revision), expected);
    assert.ok(readCount > 0, 'Snapshot reader must observe real traffic');
    return { clientMode, retryPolicy: clientMode === 'optimized' ? retryPolicy : 'legacy immediate retry; full snapshot revision read; bound >1000',
      nodeCount, writers, iterations, projectId, successfulCount: successes.length, observedCommits: commits.length,
      requestAttempts: attempts.length, conflictCount: attempts.filter(a => a.status === 409).length,
      readCount, mixedSnapshots: readerErrors.length, writerCountersVerified: writers, unchangedNodesVerified: nodeCount - 3, throughput: successes.length / ((performance.now() - start) / 1000),
      attemptMs: quantiles(attempts.map(a => a.ms)), retryToSuccessMs: quantiles(successes.map(a => a.ms)),
      maxRetries: Math.max(...successes.map(a => a.retries)), fairness: Array.from({ length: writers }, (_, writer) => {
        const samples = successes.filter(a => a.writer === writer);
        return { writer, successfulCount: samples.length, totalRetries: samples.reduce((sum, a) => sum + a.retries, 0),
          maxRetries: Math.max(...samples.map(a => a.retries)), totalOperationMs: samples.reduce((sum, a) => sum + a.ms, 0),
          retryToSuccessMs: quantiles(samples.map(a => a.ms)) };
      }), attempts, successes };
  } finally {
    reading = false;
    await reader;
    await stream.close();
    assert.equal(readerErrors.length, 0, JSON.stringify(readerErrors));
  }
}

function quantiles(values) { return { p50: percentile(values, .5), p95: percentile(values, .95), p99: percentile(values, .99) }; }
