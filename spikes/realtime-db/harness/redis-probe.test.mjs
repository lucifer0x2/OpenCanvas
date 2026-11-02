import assert from 'node:assert/strict';
import test from 'node:test';
import { decodeRedis, encodeRedis } from './redis-probe.mjs';

test('RESP probe frames binary byte lengths and partial nested replies', () => {
  assert.equal(encodeRedis(['SET', 'a', 'é']).toString(), '*3\r\n$3\r\nSET\r\n$1\r\na\r\n$2\r\né\r\n');
  const reply = Buffer.from('*3\r\n$1\r\na\r\n:7\r\n$-1\r\n');
  assert.equal(decodeRedis(reply.subarray(0, -1)), undefined);
  assert.deepEqual(decodeRedis(reply), { value: ['a', 7, null], next: reply.length });
  assert.throws(() => decodeRedis(Buffer.from('-ERR test\r\n')), /ERR test/);
});
