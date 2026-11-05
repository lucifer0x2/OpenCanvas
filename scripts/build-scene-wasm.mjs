#!/usr/bin/env node
// Pinned wasm-bindgen glue must match the crate version in crates/scene-wasm.
import { spawnSync } from "node:child_process";
import { existsSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { compressSceneWasm } from "./compress-scene-wasm.mjs";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const version = "0.2.129";
const targetDir = join(root, "target", "scene-wasm");
const toolRoot = join(targetDir, "tools");
const output = join(root, "web", "public", "scene-wasm");
const arguments_ = new Set(process.argv.slice(2));
for (const argument of arguments_) {
  if (argument !== "--install-tools")
    throw new Error(`Unknown argument: ${argument}`);
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    ...options,
  });
  if (result.error) throw result.error;
  if (result.status !== 0)
    throw new Error(`${command} exited ${result.status}`);
  return result;
}

function bindingVersion(command) {
  const result = spawnSync(command, ["--version"], { encoding: "utf8" });
  return result.status === 0 ? result.stdout.trim() : null;
}

let bindgen = process.env.WASM_BINDGEN;
if (!bindgen) {
  const local = join(toolRoot, "bin", "wasm-bindgen");
  bindgen = existsSync(local) ? local : "wasm-bindgen";
}
if (!bindingVersion(bindgen) && arguments_.has("--install-tools")) {
  // Project-local, opt-in tool installation; never modifies global Cargo binaries.
  run("cargo", [
    "install",
    "wasm-bindgen-cli",
    "--version",
    version,
    "--locked",
    "--root",
    toolRoot,
  ]);
  bindgen = join(toolRoot, "bin", "wasm-bindgen");
}
if (bindingVersion(bindgen) !== `wasm-bindgen ${version}`) {
  throw new Error(
    `Requires wasm-bindgen ${version}. Set WASM_BINDGEN to that binary, ` +
      "or run npm run build:scene-wasm -- --install-tools for a project-local install.",
  );
}

run("sh", [join(root, "scripts", "build-scene-wasm.sh"), output, targetDir], {
  env: { ...process.env, WASM_BINDGEN: bindgen },
});
compressSceneWasm(output);
console.log(`ESM, WASM and generated declarations: ${output}`);
