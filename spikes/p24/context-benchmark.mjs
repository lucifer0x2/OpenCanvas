// Usage: node spikes/p24/context-benchmark.mjs <binary> <workspace> <output.json> [mode]
// Modes: rectangles (default), repeated (manual rows), instances (equivalent master/instances).
import assert from 'node:assert/strict';
import { spawn } from 'node:child_process';
import { once } from 'node:events';
import { mkdtemp, rm, writeFile } from 'node:fs/promises';
import { connectMcp, waitFor } from '../e2e/protocol.mjs';

const [binary, workspace, output, mode = 'rectangles'] = process.argv.slice(2);
assert.ok(binary && workspace && output, 'Expected binary, workspace and output path');
assert.ok(['rectangles', 'repeated', 'instances'].includes(mode), 'Unknown benchmark mode');
const root = await mkdtemp('/private/tmp/opencanvas-p24-perf-');
const child = spawn(binary, ['--data-root', root, '--port', '0', '--workspace', workspace],
  { cwd: workspace, detached: true, stdio: ['ignore', 'pipe', 'pipe'] });
let log = '', mcp;
child.stdout.on('data', d => log += d);
child.stderr.on('data', d => log += d);
try {
  const base = await waitFor(async () => {
    assert.equal(child.exitCode, null, log);
    return log.match(/OpenCanvas (http:\/\/127\.0\.0\.1:\d+)/)?.[1];
  }, 'benchmark server');
  mcp = await connectMcp(base);
  const project = await mcp.call('create_project', { slug: 'perf-p24', name: 'Performance fixture' });
  const projectId = project.projectId ?? project.project?.projectId;
  assert.ok(projectId, JSON.stringify(project));
  const operations = [
    { op: 'create_node', id: 'page', kind: 'page', parentId: null, props: {} },
    { op: 'create_node', id: 'frame', kind: 'frame', parentId: 'page', props: { width: 1000, height: 1000 } },
  ];
  const create = (id, kind, parentId, props) => ({ op: 'create_node', id, kind, parentId, props });
  const row = { width: 100, height: 100 };
  const cell = j => ({ name: `Rectangle ${j}`, x: j * 10, y: 0, width: 20, height: 10, fill: '#112233' });
  if (mode === 'instances') {
    operations.push(create('master', 'frame', 'page', { ...row, component: { name: 'Row' } }));
    for (let j = 0; j < 5; j++) operations.push(create(`cell${j}`, 'rectangle', 'master', cell(j)));
  }
  if (mode === 'rectangles') {
    for (let i = 0; i < 6898; i++) operations.push(create(`rect${i}`, 'rectangle', 'frame', {
      name: `Rectangle ${i}`, x: i % 100, y: Math.floor(i / 100), width: 20, height: 10, fill: '#112233' }));
  } else {
    for (let i = 0; i < 1150; i++) {
      if (mode === 'instances') operations.push(create(`row${i}`, 'instance', 'frame', { componentId: 'master' }));
      else {
        operations.push(create(`row${i}`, 'frame', 'frame', row));
        for (let j = 0; j < 5; j++) operations.push(create(`row${i}-cell${j}`, 'rectangle', `row${i}`, cell(j)));
      }
    }
  }
  let revision = 0;
  for (let offset = 0; offset < operations.length; offset += 500) {
    const result = await mcp.call('apply_batch', { projectId, expectedRevision: revision,
      idempotencyKey: `batch${offset}`, operations: operations.slice(offset, offset + 500) });
    assert.ok(Number.isInteger(result.committedRevision), JSON.stringify(result));
    revision = result.committedRevision;
  }
  const args = { projectId, frameId: 'frame', compact: true, limit: 200, budget: 65536 };
  for (let i = 0; i < 5; i++) await mcp.call('get_design_context', args);
  const samplesMs = [];
  for (let i = 0; i < 30; i++) {
    const start = performance.now();
    const context = await mcp.call('get_design_context', args);
    samplesMs.push(performance.now() - start);
    assert.ok(context.nodes.length > 0);
  }
  const sorted = [...samplesMs].sort((a, b) => a - b);
  const result = { mode, authoredNodes: operations.length, expandedFrameNodes: mode === 'rectangles' ? 6899 : 6901, warmup: 5, samples: 30, p95Ms: sorted[Math.ceil(sorted.length * .95) - 1],
    medianMs: sorted[Math.floor(sorted.length / 2)], tools: mcp.tools.length,
    toolBytes: Buffer.byteLength(JSON.stringify({ tools: mcp.tools })), samplesMs };
  await writeFile(output, JSON.stringify(result, null, 2) + '\n');
  console.log(JSON.stringify(result));
} finally {
  await mcp?.close();
  if (child.exitCode === null && child.signalCode === null) {
    const exited = once(child, 'exit');
    process.kill(-child.pid, 'SIGTERM');
    await exited;
  }
  await rm(root, { recursive: true, force: true });
}
