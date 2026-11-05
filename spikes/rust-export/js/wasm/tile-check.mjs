// Ancillary check: tile geometry is a quarter, but translated rasterization need not equal a crop.
// Isolate text/image contributions without changing the painter or acceptance timing inputs.
import { readFileSync, mkdirSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const spike = fileURLToPath(new URL("../../", import.meta.url));
const root = spike + "out/wasm/tile-check/";
mkdirSync(root, { recursive: true });
const results = [];
for (const variant of ["no-text", "no-image", "neither", "full-text-mask"]) {
  const dir = root + variant;
  mkdirSync(dir, { recursive: true });
  for (const name of ["spike", "frame", "tile"]) {
    const scene = JSON.parse(readFileSync(spike + `out/wasm/scenes/${name}.json`));
    for (const n of scene.nodes) {
      if (["no-text", "neither"].includes(variant) && n.kind === "text") n.fill = "transparent";
      if (["no-image", "neither"].includes(variant) && n.kind === "image") n.kind = "frame";
    }
    writeFileSync(`${dir}/${name}.json`, JSON.stringify(scene));
  }
  const run = spawnSync(spike + "target/release/rust-spike", [
    "frame-bench", dir, "--threads", "1", "--iters", "1", "--json", dir + "/native.json",
  ], { cwd: spike, encoding: "utf8", timeout: 60_000,
    env: { ...process.env, ...(variant === "full-text-mask" ? { RUST_SPIKE_FORCE_FALLBACK: "1" } : {}) } });
  if (run.status !== 0) throw new Error(`Native tile check failed (${run.status}): ${run.stderr}`);
  const cases = JSON.parse(readFileSync(dir + "/native.json")).cases;
  results.push({ variant, cases: cases.filter((c) => c.case.startsWith("tile")).map((c) => ({ case: c.case, ...c.vsFrameCrop })) });
}
writeFileSync(root + "report.json", JSON.stringify(results, null, 2));
console.log(JSON.stringify(results, null, 2));
