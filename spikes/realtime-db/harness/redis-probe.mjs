// Tiny RESP2 test probe. Gateway implementation uses the maintained Redis crate.
import { createConnection } from 'node:net';

export function encodeRedis(args) {
  const parts = [Buffer.from(`*${args.length}\r\n`)];
  for (const value of args) { const data = Buffer.from(String(value)); parts.push(Buffer.from(`$${data.length}\r\n`), data, Buffer.from('\r\n')); }
  return Buffer.concat(parts);
}
export function decodeRedis(bytes, offset = 0) {
  const end = bytes.indexOf('\r\n', offset);
  if (end < 0) return;
  const prefix = String.fromCharCode(bytes[offset]), value = bytes.toString('utf8', offset + 1, end);
  let next = end + 2;
  if (prefix === '+' || prefix === ':') return { value: prefix === ':' ? Number(value) : value, next };
  if (prefix === '-') throw new Error(`Redis test probe: ${value}`);
  if (prefix === '$') {
    const size = Number(value);
    if (size === -1) return { value: null, next };
    if (bytes.length < next + size + 2) return;
    return { value: bytes.toString('utf8', next, next + size), next: next + size + 2 };
  }
  if (prefix === '*') {
    const values = [];
    for (let index = 0; index < Number(value); index++) {
      const item = decodeRedis(bytes, next); if (!item) return;
      values.push(item.value); next = item.next;
    }
    return { value: values, next };
  }
  throw new Error('Unsupported RESP frame in test probe');
}
export function redisProbe(url, args, timeoutMs = 2000) {
  const endpoint = new URL(url);
  const auth = endpoint.password ? ['AUTH', decodeURIComponent(endpoint.password)] : null;
  return new Promise((resolve, reject) => {
    const socket = createConnection({ host: endpoint.hostname, port: Number(endpoint.port || 6379) });
    let bytes = Buffer.alloc(0), replies = 0;
    const timer = setTimeout(() => socket.destroy(new Error('Redis test probe timeout')), timeoutMs);
    const finish = (error, value) => { clearTimeout(timer); socket.destroy(); error ? reject(error) : resolve(value); };
    socket.once('error', error => finish(error));
    socket.once('connect', () => socket.write(Buffer.concat([...(auth ? [encodeRedis(auth)] : []), encodeRedis(args)])));
    socket.on('data', chunk => {
      bytes = Buffer.concat([bytes, chunk]);
      if (bytes.length > 20 * 1024 * 1024) return finish(new Error('Redis test probe response limit'));
      try {
        let decoded;
        while ((decoded = decodeRedis(bytes))) {
          bytes = bytes.subarray(decoded.next); replies++;
          if (replies === (auth ? 2 : 1)) return finish(null, decoded.value);
        }
      } catch (error) { finish(error); }
    });
  });
}
