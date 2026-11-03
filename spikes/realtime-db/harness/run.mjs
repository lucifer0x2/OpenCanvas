import { writeFileSync } from 'node:fs';
import { resolve } from 'node:path';
import { semantics } from './semantics.mjs';
import { load } from './load.mjs';

export async function contractMatrix(urls, output, modes = (process.env.CLIENT_MODES || 'legacy,optimized').split(',')) {
  const result = { clientModes: modes, semantics: await semantics(urls), matrix: [] };
  for (const mode of modes) {
    for (const nodes of (process.env.NODE_COUNTS || '100,5000').split(',').map(Number)) {
      for (const writers of (process.env.WRITERS || '1,4,8').split(',').map(Number)) {
        const sample = await load(urls, nodes, writers, Number(process.env.ITERATIONS || 12), mode);
        writeFileSync(resolve(output, `postgres-${mode}-${nodes}-${writers}-attempts.jsonl`), sample.attempts.map(value => JSON.stringify(value)).join('\n') + '\n', { flag: 'wx' });
        writeFileSync(resolve(output, `postgres-${mode}-${nodes}-${writers}-successes.jsonl`), sample.successes.map(value => JSON.stringify(value)).join('\n') + '\n', { flag: 'wx' });
        delete sample.attempts; delete sample.successes; result.matrix.push(sample);
        console.log(JSON.stringify({ phase: 'matrix', mode, nodes, writers, success: sample.successfulCount, conflicts: sample.conflictCount }));
        writeFileSync(resolve(output, 'contract.json'), JSON.stringify(result, null, 2));
      }
    }
  }
  return result;
}
