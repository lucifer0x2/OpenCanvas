import assert from 'node:assert/strict';
import { test } from 'node:test';
import { decoder } from '../infra/pg-wire-probe.mjs';

function frame(type, payload = Buffer.alloc(0)) {
  const result = Buffer.alloc(payload.length + 5);
  result[0] = type.charCodeAt(0); result.writeUInt32BE(payload.length + 4, 1); payload.copy(result, 5);
  return result;
}
function startup(protocol = 196608) {
  const result = Buffer.alloc(8); result.writeUInt32BE(8); result.writeUInt32BE(protocol, 4); return result;
}

test('fragmented startup/authentication and coalesced extended messages keep only categories', () => {
  const observed = [];
  const feed = decoder('frontend', value => observed.push(value));
  const parse = Buffer.concat([Buffer.from('s0\0SELECT revision FROM projects WHERE project_id=$1 FOR UPDATE\0'), Buffer.alloc(2)]);
  const bytes = Buffer.concat([startup(), frame('p', Buffer.from('private-auth-secret')), frame('P', parse),
    frame('B', Buffer.from('private-bind-secret')), frame('E'), frame('S'), frame('Q', Buffer.from('COMMIT\0'))]);
  for (let offset = 0; offset < bytes.length; offset += 3) feed(bytes.subarray(offset, offset + 3));
  assert.deepEqual(observed, [{ type: 'p' }, { type: 'P', category: 'project-lock' }, { type: 'B' }, { type: 'E' }, { type: 'S' }, { type: 'Q', category: 'commit' }]);
  assert.doesNotMatch(JSON.stringify(observed), /secret|SELECT|private/);
});

test('backend authentication and ReadyForQuery use normal framed messages', () => {
  const observed = [];
  const feed = decoder('backend', value => observed.push(value));
  const bytes = Buffer.concat([frame('R', Buffer.alloc(4)), frame('Z', Buffer.from('I')), frame('D', Buffer.from('private-row'))]);
  feed(bytes.subarray(0, 2)); feed(bytes.subarray(2));
  assert.deepEqual(observed, [{ type: 'R' }, { type: 'Z' }, { type: 'D' }]);
});

test('invalid startup, TLS, unknown tags and impossible lengths fail closed', () => {
  assert.throws(() => decoder('frontend', () => {})(startup(80877103)), /plaintext/);
  assert.throws(() => decoder('backend', () => {})(frame('?')), /framing/);
  const short = frame('Q'); short.writeUInt32BE(3, 1);
  assert.throws(() => decoder('backend', () => {})(short), /framing/);
  const huge = frame('Q'); huge.writeUInt32BE(0xffffffff, 1);
  assert.throws(() => decoder('backend', () => {})(huge), /framing/);
  assert.throws(() => decoder('frontend', () => {})(Buffer.concat([startup(), frame('P', Buffer.from('missing-terminators'))])), /SQL framing/);
});
