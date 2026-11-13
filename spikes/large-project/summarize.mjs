// Task24: turn one output directory into Markdown tables (median / p95 ms).
// Usage: node spikes/large-project/summarize.mjs <output-dir>
// Reads cell-*.json, viewer-*.json and time-*.txt (/usr/bin/time -l). Prints only
// counts, sizes and timings; the private source design is never read here.
import { readFileSync, readdirSync } from "node:fs";
import { join } from "node:path";

const dir = process.argv[2];
if (!dir) {
  console.error("usage: node summarize.mjs <output-dir>");
  process.exit(2);
}
const files = readdirSync(dir);
const read = (name) => JSON.parse(readFileSync(join(dir, name), "utf8"));
const cells = files
  .filter((f) => /^cell-.*\.json$/.test(f))
  .map((f) => ({ file: f, ...read(f) }))
  .sort((a, b) => a.variant.localeCompare(b.variant) || a.copies - b.copies || a.engine.localeCompare(b.engine));

const n = (v, d = 1) => (v == null ? "–" : Number(v).toLocaleString("en-US", { maximumFractionDigits: d }));
const t = (s) => (s ? `${n(s.median)} / ${n(s.p95)}` : "–");
const mib = (b) => (b == null ? "–" : n(b / 1048576, 1));
const label = (c) => `${c.copies}× ${c.variant}`;
const table = (head, rows) =>
  [`| ${head.join(" | ")} |`, `| ${head.map(() => "---").join(" | ")} |`, ...rows.map((r) => `| ${r.join(" | ")} |`)].join("\n");
const rss = (c) => {
  const name = `time-${c.engine}-${c.variant}-${String(c.copies).padStart(2, "0")}.txt`;
  if (!files.includes(name)) return null;
  const match = readFileSync(join(dir, name), "utf8").match(/(\d+)\s+maximum resident set size/);
  return match ? Number(match[1]) : null;
};
const viewer = (c) => {
  const name = `viewer-${c.variant}-${String(c.copies).padStart(2, "0")}.json`;
  return files.includes(name) ? read(name) : null;
};
const phase = (commit, name) => commit.phases[name];
const out = [];

out.push("### Counts and sizes\n");
out.push(table(
  ["Scale", "Engine", "Authored", "Expanded", "Pages", "Instances", "Snapshot JSON MiB", "Objects body MiB (gzip)", "Rows body MiB (gzip)", "Cache admitted"],
  cells.map((c) => [label(c), c.engine, n(c.counts.authoredNodes, 0), n(c.counts.expandedNodes, 0), c.counts.pages, n(c.counts.instances, 0),
    mib(c.storage.snapshotJsonBytes), `${mib(c.http.objects.bodyBytes)} (${mib(c.http.objects.gzipBytes)})`,
    `${mib(c.http.rows.bodyBytes)} (${mib(c.http.rows.gzipBytes)})`, c.storage.cacheAdmitted ? "yes" : "no"]),
));
out.push("\n### Snapshot load and HTTP GET (median / p95 ms)\n");
out.push(table(
  ["Scale", "Engine", "Storage cold load", "Moka warm", "Encode objects", "GET objects cold", "GET objects repeat", "GET rows cold", "GET rows repeat"],
  cells.map((c) => [label(c), c.engine, t(c.storage.coldLoad), t(c.storage.mokaWarm), t(c.storage.encodeObjects),
    t(c.http.objects.getCold), t(c.http.objects.getRepeat), t(c.http.rows.getCold), t(c.http.rows.getRepeat)]),
));
for (const mode of ["strict", "scoped"]) {
  out.push(`\n### Commit, ${mode} (median / p95 ms)\n`);
  out.push(table(
    ["Scale", "Engine", "Total", "Load", "from_snapshot", "validate()", "of which themes", "of which instances (expand)", mode === "scoped" ? "Scoped check" : "History envelope", "Persist", "Cache fill"],
    cells.map((c) => {
      const m = c.commits[mode];
      const inst = phase(m, "validate.instances");
      const exp = phase(m, "validate.instances.expand");
      return [label(c), c.engine, t(m.total), t(phase(m, "load")), t(phase(m, "candidate.from_snapshot")), t(phase(m, "candidate.validate")),
        t(phase(m, "validate.theme")), `${t(inst)} (${exp ? n(exp.median) : "–"})`,
        t(phase(m, mode === "scoped" ? "scoped.check" : "history.envelope")), t(phase(m, "persist")), t(phase(m, "cache.fill"))];
    }),
  ));
}
out.push("\n### Reads (median / p95 ms)\n");
out.push(table(
  ["Scale", "Engine", "expand() whole project", "find_nodes name, many", "find_nodes name, none", "find_nodes text", "get_outline page", "get_design_context frame"],
  cells.map((c) => [label(c), c.engine, t(c.expandWholeProject), t(c.reads.findNameManyMatches.time), t(c.reads.findNameNoMatch.time),
    t(c.reads.findTextContains.time), t(c.reads.outlinePage.time), t(c.reads.designContextFrame.time)]),
));
out.push("\n### Viewer cost in Node and peak RSS\n");
out.push(table(
  ["Scale", "Engine", "JSON.parse ms", "snapshotNodeIndex ms", "Retained heap MiB", "Peak RSS MiB", "RSS after seed MiB"],
  cells.map((c) => {
    const v = c.engine === "sqlite" ? viewer(c) : null;
    return [label(c), c.engine, v ? t(v.parseMs) : "–", v ? t(v.indexMs) : "–", v ? n(v.retainedHeapMiB.median) : "–",
      mib(rss(c)), c.rssAfterSeedKib ? n(c.rssAfterSeedKib / 1024, 0) : "–"];
  }),
));
console.log(out.join("\n"));
