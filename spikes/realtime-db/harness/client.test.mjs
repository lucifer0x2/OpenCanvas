import { test } from 'node:test';
import assert from 'node:assert/strict';
import { createServer } from 'node:http';
import { percentile, subscribe } from './client.mjs';

test('quantiles require actual samples and use nearest rank', () => {
  assert.throws(() => percentile([], .95), /Empty/);
  assert.equal(percentile([100, 1, 20, 3], .5), 3);
  assert.equal(percentile([100, 1, 20, 3], .99), 100);
});

test('SSE parser handles split packets and excludes heartbeat frames', async () => {
  const server = createServer((_request, response) => {
    response.writeHead(200, { 'content-type': 'text/event-stream' });
    response.write(': heartbeat\n\ndata: {"revision":');
    setTimeout(() => { response.write('1}\n\nid: p:2\ndata: {"revision":2}\n\n'); }, 10);
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const stream = subscribe(`http://127.0.0.1:${server.address().port}`, 'p');
  try { assert.deepEqual((await stream.until(2)).map(e => e.revision), [1, 2]); }
  finally { await stream.close(); server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
});

test('SSE missing events fail instead of green empty traffic', async () => {
  const server = createServer((_request, response) => {
    response.writeHead(200, { 'content-type': 'text/event-stream' });
    response.write(': heartbeat\n\n');
  });
  await new Promise(resolve => server.listen(0, '127.0.0.1', resolve));
  const stream = subscribe(`http://127.0.0.1:${server.address().port}`, 'p');
  try { await assert.rejects(stream.until(1, 30), /missing commits/); }
  finally { await stream.close(); server.closeAllConnections(); await new Promise(resolve => server.close(resolve)); }
});
