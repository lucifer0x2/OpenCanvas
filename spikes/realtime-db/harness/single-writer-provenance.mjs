import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { relative, resolve } from 'node:path';

export const sha256 = bytes => createHash('sha256').update(bytes).digest('hex');
function files(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    if (['target', 'node_modules', '.runtime', '.git'].includes(entry.name)) return [];
    const path = resolve(directory, entry.name);
    return entry.isDirectory() ? files(path) : [path];
  });
}

export function sourceManifest(source) {
  const sources = ['core', 'postgres', 'harness', 'infra'].flatMap(name => files(resolve(source, name))).sort();
  const entries = sources.map(path => ({ path: relative(source, path), sha256: sha256(readFileSync(path)) }));
  return { fingerprint: sha256(JSON.stringify(entries)), files: entries };
}

export function freeze(source, executable, harnessRoot) {
  const harness = ['harness/single-writer.mjs', 'harness/single-writer-load.mjs',
    'harness/single-writer-provenance.mjs', 'harness/single-writer-profiles.mjs',
    'harness/client.mjs', 'harness/retry.mjs', 'harness/load.mjs',
    'infra/runtime.mjs', 'infra/gateways.mjs'];
  return { source: sourceManifest(source), binary: { sha256: sha256(readFileSync(executable)) },
    driver: harness.map(path => ({ path, sha256: sha256(readFileSync(resolve(harnessRoot, path))) })),
    rustc: execFileSync('rustc', ['--version'], { encoding: 'utf8' }).trim(),
    cargo: execFileSync('cargo', ['--version'], { encoding: 'utf8' }).trim(), node: process.version };
}

export function assertFrozen(before, after) {
  assert.deepEqual(after, before, 'Source, executable, driver and toolchain must remain frozen during measurement');
}
