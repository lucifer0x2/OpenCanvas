import assert from 'node:assert/strict';
import { readFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { freeze, sha256 } from './single-writer-provenance.mjs';

const files = ['harness/task17-benchmark.mjs', 'harness/task17-profiles.mjs', 'harness/task17-provenance.mjs',
  'infra/redis-runtime.mjs'];
export function freezeTask17(source, binary, root) {
  return { ...freeze(source, binary, root), task17Driver: files.map(path => ({ path, sha256: sha256(readFileSync(resolve(root, path))) })) };
}
export function frozenTask17(before, after) { assert.deepEqual(after, before, 'Source/binary/client/driver/toolchain frozen'); }
