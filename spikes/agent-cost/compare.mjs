// Compare two agent-cost runs as a Markdown table.
//   node spikes/agent-cost/compare.mjs baseline after-07B [--all]
// Arguments are labels under artifacts/agent-cost/ or paths to .json files.
// Totals are always listed, phases as calls/bytes/callMs, numeric `result`
// facts only when they changed; --all lists every numeric field. Delta % is relative to the first file.
import { readFileSync } from "node:fs";

const args = process.argv.slice(2);
const all = args.includes("--all");
const [left, right] = args.filter((a) => a !== "--all");
if (!left || !right) {
  console.error(
    "Usage: node spikes/agent-cost/compare.mjs <before> <after> [--all]",
  );
  process.exit(2);
}

function load(name) {
  const path = name.endsWith(".json")
    ? name
    : new URL(`../../artifacts/agent-cost/${name}.json`, import.meta.url);
  return JSON.parse(readFileSync(path, "utf8"));
}

/** Flattens numeric leaves: {a:{b:1}, c:[2]} → [["a.b",1],["c.0",2]]. */
function numbers(value, prefix = "", out = []) {
  if (typeof value === "number") out.push([prefix, value]);
  else if (value && typeof value === "object")
    for (const [key, child] of Object.entries(value))
      numbers(child, prefix ? `${prefix}.${key}` : key, out);
  return out;
}

function delta(a, b) {
  if (a === undefined || b === undefined) return "n/a";
  const d = Math.round((b - a) * 100) / 100;
  const pct =
    a === 0
      ? b === 0
        ? "0%"
        : "new"
      : `${d >= 0 ? "+" : ""}${((d / a) * 100).toFixed(1)}%`;
  return `${d >= 0 ? "+" : ""}${d} (${pct})`;
}

const before = load(left);
const after = load(right);
console.log(
  `Comparing \`${before.label}\` (${before.generatedAt}) → \`${after.label}\` (${after.generatedAt})`,
);
if (Object.keys(after.overrides ?? {}).length)
  console.log(
    `Overrides in ${after.label}: \`${JSON.stringify(after.overrides)}\``,
  );
for (const run of [before, after])
  if (!run.ok) console.log(`WARNING: ${run.label} has failed scenarios`);
console.log(
  "\n| Scenario | Metric | Before | After | Delta |\n| --- | --- | ---: | ---: | ---: |",
);
const ids = [
  ...new Set([
    ...Object.keys(before.scenarios),
    ...Object.keys(after.scenarios),
  ]),
];
for (const id of ids) {
  const a = before.scenarios[id] ?? {};
  const b = after.scenarios[id] ?? {};
  const rows = [];
  for (const section of ["totals", "phases"]) {
    const left = new Map(numbers(a[section]));
    const right = new Map(numbers(b[section]));
    for (const key of new Set([...left.keys(), ...right.keys()]))
      rows.push([
        `${section}.${key}`,
        left.get(key),
        right.get(key),
        all || section === "totals" || /\.(calls|bytes|callMs)$/.test(key),
      ]);
  }
  const left = new Map(numbers(a.result));
  const right = new Map(numbers(b.result));
  for (const key of new Set([...left.keys(), ...right.keys()]))
    rows.push([
      `result.${key}`,
      left.get(key),
      right.get(key),
      all || left.get(key) !== right.get(key),
    ]);
  for (const [metric, x, y, show] of rows)
    if (show)
      console.log(
        `| ${id} | ${metric} | ${x ?? "—"} | ${y ?? "—"} | ${delta(x, y)} |`,
      );
}
