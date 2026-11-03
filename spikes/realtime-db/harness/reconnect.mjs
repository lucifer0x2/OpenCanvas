import { performance } from 'node:perf_hooks';
import { pause } from './client.mjs';

export function reconnectingStream(url, projectId, initialRevision) {
  const controller = new AbortController();
  const events = [], errors = [], gapsMs = [];
  let cursor = initialRevision, disconnectedAt, attempts = 0;
  const task = (async () => {
    while (!controller.signal.aborted) {
      attempts++;
      try {
        const response = await fetch(`${url}/events/${projectId}?sinceRevision=${cursor}`, { signal: controller.signal });
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        if (disconnectedAt !== undefined) { gapsMs.push(performance.now() - disconnectedAt); disconnectedAt = undefined; }
        let buffer = '';
        const decoder = new TextDecoder();
        for await (const chunk of response.body) {
          buffer += decoder.decode(chunk, { stream: true }).replace(/\r/g, '');
          let index;
          while ((index = buffer.indexOf('\n\n')) >= 0) {
            const frame = buffer.slice(0, index); buffer = buffer.slice(index + 2);
            const data = frame.split('\n').filter(line => line.startsWith('data:')).map(line => line.slice(5).trim()).join('\n');
            if (data) { const event = JSON.parse(data); events.push(event); cursor = Math.max(cursor, event.revision); }
          }
        }
        if (!controller.signal.aborted) throw new Error('SSE EOF');
      } catch (error) {
        if (!controller.signal.aborted) {
          errors.push({ attempt: attempts, cursor, error: error.message });
          disconnectedAt ??= performance.now(); await pause(30);
        }
      }
    }
  })();
  return { events, errors, gapsMs, get attempts() { return attempts; },
    async close() { controller.abort(); await task; } };
}
