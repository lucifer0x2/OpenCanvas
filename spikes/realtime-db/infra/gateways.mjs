import { spawn } from 'node:child_process';
import { randomBytes } from 'node:crypto';
import { openSync, readFileSync, closeSync } from 'node:fs';
import { resolve } from 'node:path';
import { performance } from 'node:perf_hooks';
import { pause, request } from '../harness/client.mjs';

export class Gateways {
  constructor(runtime, executable) {
    this.runtime = runtime; this.executable = executable; this.entries = []; this.sequence = 0; this.poolId = randomBytes(4).toString('hex');
  }
  get urls() { return this.entries.map(entry => entry.url); }
  async start(index, fixedPort) {
    const logPath = resolve(this.runtime.directory, `gateway-${this.poolId}-${index}-${this.sequence++}.log`);
    const log = openSync(logPath, 'a', 0o600);
    const child = spawn(this.executable, [], { env: { ...process.env, ...this.runtime.env, PORT: String(fixedPort || 0) },
      stdio: ['ignore', log, log] });
    closeSync(log);
    let exited = false;
    child.once('exit', () => { exited = true; });
    const start = performance.now();
    for (let n = 0; n < 300; n++) {
      const line = readFileSync(logPath, 'utf8').split('\n').find(value => value.startsWith('{') && value.includes('"ready":true'));
      if (line) {
        const ready = JSON.parse(line);
        const entry = { child, pid: child.pid, port: ready.port, url: `http://127.0.0.1:${ready.port}`, logPath };
        this.entries[index] = entry;
        if ((await request(entry.url, '/health')).status === 200) return { pid: entry.pid, url: entry.url, startupMs: performance.now() - start };
      }
      if (exited) throw new Error(`Gateway exited before readiness; inspect private ${logPath}`);
      await pause(100);
    }
    child.kill('SIGKILL'); throw new Error('Gateway startup timeout');
  }
  async kill(index, signal = 'SIGKILL') {
    const child = this.entries[index]?.child;
    if (!child || child.exitCode !== null || child.signalCode !== null) return;
    const done = new Promise(resolve => child.once('exit', resolve)); child.kill(signal); await done;
  }
  async restart(index, downtimeMs = 150) {
    const port = this.entries[index].port;
    const start = performance.now();
    await this.kill(index); await pause(downtimeMs);
    const result = await this.start(index, port);
    return { ...result, totalRestartMs: performance.now() - start };
  }
  async close() { for (let index = 0; index < this.entries.length; index++) await this.kill(index, 'SIGTERM'); }
}
