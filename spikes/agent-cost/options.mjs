// ─── EXTENSION POINT ────────────────────────────────────────────────────────
// Every scenario reads its knobs from here. The defaults below ARE the baseline:
// they use only tools and arguments that exist before Task 07B–07F. A later
// slice opts into new parameters from the CLI, never by editing a default:
//
//   node spikes/agent-cost/run.mjs --label after-07C --opt s2.contextArgs='{"compact":true}'
//   node spikes/agent-cost/run.mjs --label after-07E --opt s3.pollArgs='{"waitMs":2000}'
//   node spikes/agent-cost/run.mjs --label after-07B --opt s3.inspectArgs='{"fields":["bounds"]}'
//   node spikes/agent-cost/run.mjs --label after-06  --opt s1.strategy=findNodes
//
// `*Args` objects are merged verbatim into the named tool call. `strategy` picks
// an entry from that scenario module's `strategies` table; a slice that adds a
// new tool adds a strategy there. The chosen options are written to the output
// file so a comparison always shows what differed.
// ────────────────────────────────────────────────────────────────────────────
export const DEFAULTS = Object.freeze({
  s1: { strategy: "outlineThenSubtree", subtreeArgs: {} },
  s2: { contextArgs: {} },
  s3: { pollIntervalMs: 250, renderArgs: {}, pollArgs: {}, inspectArgs: {} },
  s4: { strategy: "rereadOutlineAndContext" },
  s5: { samples: 20, extraProjects: 19, outlineArgs: {} },
  s6: { nodes: 5000, outlineArgs: {}, contextArgs: {} },
});

const SCENARIOS = ["S1", "S2", "S3", "S4", "S5", "S6"];

function parseValue(raw) {
  try {
    return JSON.parse(raw);
  } catch {
    return raw;
  }
}

/** Parses `--label`, `--only S1,S3`, `--build` and repeated `--opt path=value`. */
export function parseArgs(argv) {
  const options = structuredClone(DEFAULTS);
  const overrides = {};
  let label = "baseline";
  let only = SCENARIOS;
  let build = false;
  for (let i = 0; i < argv.length; i++) {
    const arg = argv[i];
    if (arg === "--label") label = argv[++i];
    else if (arg === "--only")
      only = argv[++i].split(",").map((s) => s.trim().toUpperCase());
    else if (arg === "--build") build = true;
    else if (arg === "--opt") {
      const spec = argv[++i] ?? "";
      const eq = spec.indexOf("=");
      if (eq < 1) throw new Error(`--opt expects path=value, got "${spec}"`);
      const path = spec.slice(0, eq).split(".");
      const value = parseValue(spec.slice(eq + 1));
      let target = options;
      for (const key of path.slice(0, -1)) {
        if (typeof target[key] !== "object")
          throw new Error(`Unknown option path ${spec}`);
        target = target[key];
      }
      if (!(path.at(-1) in target))
        throw new Error(`Unknown option ${path.join(".")}`);
      target[path.at(-1)] = value;
      overrides[path.join(".")] = value;
    } else throw new Error(`Unknown argument ${arg}`);
  }
  if (!/^[a-z0-9][a-z0-9._-]*$/i.test(label ?? ""))
    throw new Error(`Invalid label "${label}"`);
  for (const id of only)
    if (!SCENARIOS.includes(id)) throw new Error(`Unknown scenario ${id}`);
  return { label, only, build, options, overrides };
}
