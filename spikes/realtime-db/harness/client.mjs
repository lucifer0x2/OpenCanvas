import { randomUUID } from 'node:crypto';
import { performance } from 'node:perf_hooks';

export const key = () => randomUUID();
export const pause = ms => new Promise(resolve => setTimeout(resolve, ms));

export async function request(base, path, body, headers = {}, timeoutMs = 30000) {
  const start = performance.now();
  const response = await fetch(`${base}${path}`, {
    method: body === undefined ? 'GET' : 'POST',
    headers: { 'content-type': 'application/json', ...headers },
    body: body === undefined ? undefined : JSON.stringify(body),
    signal: AbortSignal.timeout(timeoutMs),
  }).catch(error => { throw new Error(`${path}: ${error.message}`, { cause: error }); });
  return { status: response.status, body: await response.json(), ms: performance.now() - start };
}

export const batch = (projectId, expectedRevision, operations, extra = {}) => ({
  projectId, expectedRevision, idempotencyKey: key(), operations, ...extra,
});
export const create = (id, parentId = null, kind = 'page', props = {}) => ({
  op: 'create_node', id, parentId, kind, props,
});
export const update = (id, props) => ({ op: 'update_node', id, props });

export function subscribe(base, projectId, sinceRevision = 0) {
  const controller = new AbortController();
  const events = [];
  let error;
  let resolveReady;
  const ready = new Promise(resolve => { resolveReady = resolve; });
  const task = (async () => {
    const response = await fetch(`${base}/events/${projectId}?sinceRevision=${sinceRevision}`, { signal: controller.signal });
    if (!response.ok) throw new Error(`SSE HTTP ${response.status}`);
    resolveReady();
    const decoder = new TextDecoder();
    let buffer = '';
    for await (const chunk of response.body) {
      buffer += decoder.decode(chunk, { stream: true }).replace(/\r/g, '');
      let boundary;
      while ((boundary = buffer.indexOf('\n\n')) >= 0) {
        const frame = buffer.slice(0, boundary);
        buffer = buffer.slice(boundary + 2);
        const data = frame.split('\n').filter(line => line.startsWith('data:')).map(line => line.slice(5).trim()).join('\n');
        if (data) events.push(JSON.parse(data));
      }
    }
  })().catch(e => { if (!controller.signal.aborted) error = e; resolveReady(); });
  return { events, ready, async until(count, timeoutMs = 5000) {
    const until = performance.now() + timeoutMs;
    while (events.length < count && performance.now() < until) { if (error) throw error; await pause(10); }
    if (events.length < count) throw new Error(`SSE missing commits: wanted ${count}, got ${events.length}`);
    return events;
  }, async close() { controller.abort(); await task; } };
}

export function percentile(values, p) {
  if (!values.length) throw new Error('Empty latency sample');
  const sorted = [...values].sort((a, b) => a - b);
  return sorted[Math.min(sorted.length - 1, Math.ceil(p * sorted.length) - 1)];
}
