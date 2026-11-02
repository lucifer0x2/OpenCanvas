import { createHash } from 'node:crypto';
import { execFileSync } from 'node:child_process';
import { readFileSync, readdirSync } from 'node:fs';
import { relative, resolve } from 'node:path';
import { root } from './runtime.mjs';

const hash = bytes => createHash('sha256').update(bytes).digest('hex');
function files(directory) {
  return readdirSync(directory, { withFileTypes: true }).flatMap(entry => {
    if (['target', 'node_modules', '.runtime'].includes(entry.name)) return [];
    const path = resolve(directory, entry.name);
    return entry.isDirectory() ? files(path) : [path];
  });
}
export function provenance() {
  const sources = ['core', 'postgres', 'harness', 'infra'].flatMap(name => files(resolve(root, name))).sort();
  const sourceFiles = sources.map(path => ({ path: relative(root, path), sha256: hash(readFileSync(path)) }));
  return { baseline: execFileSync('git', ['rev-parse', 'HEAD'], { cwd: root, encoding: 'utf8' }).trim(),
    sourceFingerprint: hash(JSON.stringify(sourceFiles)), sourceFiles,
    binaries: ['postgres/target/debug/oc-spike-postgres', '.runtime/v2-target/debug/oc-spike-postgres'].map(path => ({
      path, sha256: hash(readFileSync(resolve(root, path))) })),
    rustc: execFileSync('rustc', ['--version'], { encoding: 'utf8' }).trim(),
    cargo: execFileSync('cargo', ['--version'], { encoding: 'utf8' }).trim() };
}
