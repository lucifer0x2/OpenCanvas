// Task24 item 6: browser-side cost of fetching the whole project, measured in Node.
// Usage: node --expose-gc spikes/large-project/viewer-cost.mjs <body.json> [warmup] [samples]
// Times JSON.parse of the GET /snapshot object body and the viewer's
// snapshotNodeIndex build (a new revision each run, as after a commit refetch),
// then reports the heap retained by the parsed snapshot plus its index
// (heap after GC with both alive, minus the heap before the first parse).
import { readFileSync } from "node:fs";
import { performance } from "node:perf_hooks";
import { snapshotNodeIndex } from "../../web/src/nodeIndex.ts";

const [file, warmupArg = "1", samplesArg = "5"] = process.argv.slice(2);
if (!file || typeof globalThis.gc !== "function") {
  console.error("usage: node --expose-gc viewer-cost.mjs <body.json> [warmup] [samples]");
  process.exit(2);
}
const warmup = Number(warmupArg);
const samples = Number(samplesArg);
const text = readFileSync(file, "utf8");

const parse = [];
const index = [];
const retained = [];
let nodes = 0;
let keep = null;
globalThis.gc();
const baseline = process.memoryUsage().heapUsed;
for (let run = 0; run < warmup + samples; run += 1) {
  keep = null;
  globalThis.gc();
  const start = performance.now();
  const snapshot = JSON.parse(text);
  const parsed = performance.now();
  snapshot.revision = run + 1; // newer revision: the cached index is rebuilt
  const built = snapshotNodeIndex(snapshot);
  const indexed = performance.now();
  keep = [snapshot, built];
  globalThis.gc();
  const after = process.memoryUsage().heapUsed;
  nodes = built.byId.size;
  if (run >= warmup) {
    parse.push(parsed - start);
    index.push(indexed - parsed);
    retained.push((after - baseline) / (1024 * 1024));
  }
}

// Nearest rank: sorted[ceil(p*N)-1].
const summary = (values) => {
  const sorted = [...values].sort((a, b) => a - b);
  const rank = (p) => sorted[Math.max(1, Math.ceil(p * sorted.length)) - 1];
  return { median: rank(0.5), p95: rank(0.95), min: sorted[0], max: sorted.at(-1), n: values.length, values };
};

console.log(JSON.stringify({
  bodyBytes: Buffer.byteLength(text),
  nodes,
  parseMs: summary(parse),
  indexMs: summary(index),
  retainedHeapMiB: summary(retained),
  node: process.version,
}));
