// Agent-cost harness (Task 07A). Usage, from the repository root:
//   node spikes/agent-cost/run.mjs [--label baseline] [--only S1,S3] [--build] [--opt s2.contextArgs='{...}']
// Writes artifacts/agent-cost/<label>.json. See README.md for the schema.
import { execFileSync } from "node:child_process";
import { createHash } from "node:crypto";
import { mkdir, readFile, stat, writeFile } from "node:fs/promises";
import { workspace } from "../e2e/server.mjs";
import { installMeter, responseBytes } from "./meter.mjs";
import { parseArgs } from "./options.mjs";
import * as s1 from "./scenarios/s1-find-text.mjs";
import * as s2 from "./scenarios/s2-design-context.mjs";
import * as s3 from "./scenarios/s3-render-inspect.mjs";
import * as s4 from "./scenarios/s4-conflict.mjs";
import * as s5 from "./scenarios/s5-outline-projects.mjs";
import * as s6 from "./scenarios/s6-large.mjs";

const SCENARIOS = [s1, s2, s3, s4, s5, s6];
const BINARY = `${workspace}target/debug/opencanvas-server`;

function build() {
  const run = (cmd, args) =>
    execFileSync(cmd, args, { cwd: workspace, stdio: "inherit" });
  run("npm", ["run", "build:renderer"]);
  run("cargo", [
    "build",
    "--offline",
    "-p",
    "opencanvas-layout",
    "-p",
    "opencanvas-server",
  ]);
}

async function environment() {
  const git = (...args) => {
    try {
      return execFileSync("git", args, {
        cwd: workspace,
        encoding: "utf8",
      }).trim();
    } catch {
      return null;
    }
  };
  const binary = await readFile(BINARY);
  return {
    node: process.version,
    platform: `${process.platform}-${process.arch}`,
    gitHead: git("rev-parse", "--short", "HEAD"),
    serverBinary: {
      sha256: createHash("sha256").update(binary).digest("hex").slice(0, 16),
      modified: (await stat(BINARY)).mtime.toISOString(),
    },
  };
}

/** Size of the MCP handshake an agent pays once per session (instructions + tool schemas). */
function handshake(exchanges) {
  const pick = (method) =>
    exchanges.find((e) => e.method === method && e.chunks.length);
  const init = pick("initialize");
  const tools = pick("tools/list");
  return {
    initializeResponseBytes: init ? responseBytes(init) : null,
    toolsListResponseBytes: tools ? responseBytes(tools) : null,
  };
}

const {
  label,
  only,
  build: rebuild,
  options,
  overrides,
} = parseArgs(process.argv.slice(2));
if (rebuild) build();
const output = {
  schema: "opencanvas.agent-cost/1",
  label,
  generatedAt: new Date().toISOString(),
  ok: false,
  environment: await environment(),
  options,
  overrides,
  handshake: null,
  summary: {},
  scenarios: {},
};
const meter = installMeter();
let failed = false;
try {
  for (const scenario of SCENARIOS.filter((s) => only.includes(s.id))) {
    const t0 = performance.now();
    process.stdout.write(`${scenario.id} ${scenario.title} … `);
    try {
      const result = await scenario.run(meter, options);
      output.handshake ??= handshake(meter.exchanges);
      output.scenarios[scenario.id] = result;
      output.summary[scenario.id] = result.totals;
      const t = result.totals;
      console.log(
        `${t.calls} calls, ${t.bytes} B, wall ${t.wallMs} ms (${Math.round(performance.now() - t0)} ms incl. setup)`,
      );
    } catch (error) {
      failed = true;
      output.scenarios[scenario.id] = {
        id: scenario.id,
        title: scenario.title,
        error: String(error?.stack ?? error),
      };
      console.log(`FAILED\n${error?.stack ?? error}`);
    } finally {
      meter.exchanges.length = 0;
    }
  }
} finally {
  meter.restore();
}
output.ok = !failed;
const dir = new URL("../../artifacts/agent-cost/", import.meta.url);
await mkdir(dir, { recursive: true });
const file = new URL(`${label}.json`, dir);
await writeFile(file, `${JSON.stringify(output, null, 2)}\n`);
console.log(`${failed ? "Completed with failures" : "Wrote"} ${file.pathname}`);
process.exitCode = failed ? 1 : 0;
