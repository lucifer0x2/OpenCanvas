import { createConnection, createServer } from 'node:net';

const allowed = { frontend: new Set('BCDEFHPpQSXdcf'), backend: new Set('RK SZTD CENAI nt123sGHWVdcfv'.replaceAll(' ', '')) };
const maximumFrameBytes = 64 * 1024 * 1024;

function category(sql) {
  const text = sql.replace(/\s+/g, ' ').trim().toUpperCase();
  if (/^(BEGIN|START TRANSACTION)/.test(text)) return 'begin';
  if (/^(COMMIT|ROLLBACK)/.test(text)) return text.split(' ')[0].toLowerCase();
  if (text.includes('PG_NOTIFY')) return 'notify';
  if (text.includes('FROM PROJECTS') && text.includes('FOR UPDATE')) return 'project-lock';
  if (text.includes('FROM IDEMPOTENCY')) return 'idempotency-read';
  if (text.includes('FROM NODES')) return 'candidate-read';
  for (const [expression, name] of [[/INSERT INTO NODES/, 'node-upsert'], [/DELETE FROM NODES/, 'node-delete'],
    [/UPDATE PROJECTS/, 'project-update'], [/INSERT INTO COMMITS/, 'commit-insert'], [/INSERT INTO IDEMPOTENCY/, 'idempotency-insert']]) {
    if (expression.test(text)) return name;
  }
  return 'other';
}

// A transparent framing observer. Authentication, Bind values and raw payloads
// are never emitted, persisted or included in errors. TLS is intentionally unsupported.
export function decoder(direction, onFrame) {
  if (!allowed[direction]) throw new Error('Unknown PostgreSQL protocol direction');
  let pending = Buffer.alloc(0);
  let startup = direction === 'frontend';
  return chunk => {
    pending = Buffer.concat([pending, chunk]);
    while (pending.length) {
      if (startup) {
        if (pending.length < 4) return;
        const length = pending.readUInt32BE();
        if (length < 8 || length > maximumFrameBytes) throw new Error('Invalid PostgreSQL startup framing');
        if (pending.length < length) return;
        if (pending.readUInt32BE(4) !== 196608) throw new Error('Only PostgreSQL v3 plaintext startup is supported');
        pending = pending.subarray(length); startup = false;
        continue;
      }
      if (pending.length < 5) return;
      const type = String.fromCharCode(pending[0]);
      const length = pending.readUInt32BE(1);
      if (!allowed[direction].has(type) || length < 4 || length > maximumFrameBytes) throw new Error('Invalid PostgreSQL message framing');
      if (pending.length < length + 1) return;
      const frame = { type };
      if (direction === 'frontend' && (type === 'P' || type === 'Q')) {
        const payload = pending.subarray(5, length + 1);
        const start = type === 'P' ? payload.indexOf(0) + 1 : 0;
        const end = payload.indexOf(0, start);
        if ((type === 'P' && start === 0) || end < start) throw new Error('Invalid PostgreSQL SQL framing');
        frame.category = category(payload.subarray(start, end).toString('utf8'));
      }
      onFrame(frame);
      pending = pending.subarray(length + 1);
    }
  };
}

export async function wireProbe(port) {
  let observation = { frontend: {}, backend: {}, statements: {} };
  let failure;
  const sockets = new Set();
  const observe = direction => frame => {
    const counts = observation[direction];
    counts[frame.type] = (counts[frame.type] || 0) + 1;
    if (frame.category) observation.statements[frame.category] = (observation.statements[frame.category] || 0) + 1;
  };
  const server = createServer(client => {
    const upstream = createConnection({ host: '127.0.0.1', port });
    for (const socket of [client, upstream]) { sockets.add(socket); socket.on('close', () => sockets.delete(socket)); }
    const fail = error => { failure ||= error; client.destroy(); upstream.destroy(); };
    client.on('error', fail); upstream.on('error', fail);
    for (const [input, output, direction] of [[client, upstream, 'frontend'], [upstream, client, 'backend']]) {
      const decode = decoder(direction, observe(direction));
      input.on('data', bytes => { try { decode(bytes); } catch (error) { fail(error); } });
      input.pipe(output);
    }
  });
  await new Promise((resolve, reject) => { server.once('error', reject); server.listen(0, '127.0.0.1', resolve); });
  return { port: server.address().port,
    checkpoint() {
      if (failure) throw failure;
      observation = { frontend: {}, backend: {}, statements: {} };
    },
    result() {
      if (failure) throw failure;
      const result = structuredClone(observation);
      result.sqlExecutions = (result.frontend.Q || 0) + (result.frontend.E || 0);
      result.synchronizationCycles = (result.frontend.Q || 0) + (result.frontend.S || 0);
      result.readyForQuery = result.backend.Z || 0;
      return result;
    },
    async close() {
      for (const socket of sockets) socket.destroy();
      await new Promise(resolve => server.close(resolve));
    } };
}
