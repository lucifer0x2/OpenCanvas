import assert from 'node:assert/strict';
import { performance } from 'node:perf_hooks';
import { batch, pause, request } from './client.mjs';

export const retryPolicy = Object.freeze({ maxConflicts: 1000, maxNetworkRetries: 3, baseMs: 2, capMs: 64 });

// Equal jitter avoids zero-delay retry storms while retaining independent writers.
export function retryDelay(retry, random = Math.random, policy = retryPolicy) {
  const ceiling = Math.min(policy.capMs, policy.baseMs * 2 ** Math.min(retry, 30));
  return ceiling / 2 + random() * ceiling / 2;
}

// This rebase is only for the benchmark's known independent fixture assignments.
// It is not a general command merge/retry policy for production authoring intent.
export async function commitFixture({ url, projectId, operations, onAttempt = () => {},
  send = request, sleep = pause, random = Math.random, policy = retryPolicy }) {
  let retries = 0, networkRetries = 0, sleepMs = 0, revisionReads = 0, revisionReadMs = 0;
  const wait = async retry => { const ms = retryDelay(retry, random, policy); sleepMs += ms; await sleep(ms); };
  while (true) {
    const current = await send(url, `/projects/${projectId}/revision`);
    revisionReads++; revisionReadMs += current.ms;
    assert.equal(current.status, 200, JSON.stringify(current.body));
    assert.equal(current.body.projectId, projectId);
    assert.ok(Number.isInteger(current.body.revision));
    const payload = batch(projectId, current.body.revision, operations);
    let result;
    for (let uncertainRetry = 0; ; uncertainRetry++) {
      const start = performance.now();
      try {
        result = await send(url, '/batch', payload);
      } catch (error) {
        onAttempt({ retry: retries, networkRetry: uncertainRetry, key: payload.idempotencyKey,
          expectedRevision: payload.expectedRevision, status: 0, ms: performance.now() - start, error: 'NETWORK_ERROR' });
        if (uncertainRetry >= policy.maxNetworkRetries) throw error;
        networkRetries++;
        await wait(uncertainRetry);
        continue; // Unknown outcome: preserve the exact body AND idempotency key.
      }
      onAttempt({ retry: retries, networkRetry: uncertainRetry, key: payload.idempotencyKey,
        expectedRevision: payload.expectedRevision, status: result.status, ms: result.ms, error: result.body.error?.code });
      break;
    }
    if (result.status === 409 && result.body.error?.code === 'REVISION_CONFLICT') {
      if (retries >= policy.maxConflicts) throw new Error('Conflict retry bound exceeded');
      await wait(retries++);
      continue; // A definite rejection permits a fresh revision/key for this fixture.
    }
    assert.equal(result.status, 200, JSON.stringify(result.body));
    return { retries, networkRetries, sleepMs, revisionReads, revisionReadMs, revision: result.body.committedRevision };
  }
}
