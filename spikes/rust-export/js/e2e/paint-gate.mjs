// Part D pixel gate for the optimized painter (src/gate.rs, `rust-spike paint-gate`), before any
// Rust timing: on the captured inputs (25 viewport widths × scale 1, 2) and the Part B scene, the
// bbox painter must equal the original full-canvas painter and itself at 1, 2 and N threads, byte
// for byte. A difference exits non-zero and stops the run.
import { execFileSync } from "node:child_process";
import path from "node:path";
import { FRAMES } from "./frames.mjs";
import { SPIKE, SPIKE_BIN, readJson, work } from "./common.mjs";

export function paintGate() {
  const json = work("results/paint-gate.json");
  const inputs = FRAMES.map((f) => work(`inputs/${f.name}.json`));
  execFileSync(SPIKE_BIN, ["paint-gate", ...inputs, "--scene", path.join(SPIKE, "data/scene.json"),
    "--image", path.join(SPIKE, "data/asset.png"), "--json", json], { stdio: "inherit" });
  return readJson(json);
}
