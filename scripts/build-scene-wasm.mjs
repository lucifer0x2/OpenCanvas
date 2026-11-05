#!/usr/bin/env node
// Pinned wasm-bindgen glue must match the crate version in crates/scene-wasm.
import { spawnSync } from "node:child_process";
import { existsSync, mkdirSync, readFileSync, rmSync } from "node:fs";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

const root = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const version = "0.2.129";
const targetDir = join(root, "target", "scene-wasm");
const toolRoot = join(targetDir, "tools");
const output = join(root, "web", "public", "scene-wasm");
const arguments_ = new Set(process.argv.slice(2));
for (const argument of arguments_) {
  if (argument !== "--install-tools") throw new Error(`Unknown argument: ${argument}`);
}

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    cwd: root,
    stdio: "inherit",
    ...options,
  });
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Error(`${command} exited ${result.status}`);
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
  run("cargo", ["install", "wasm-bindgen-cli", "--version", version,
    "--locked", "--root", toolRoot]);
  bindgen = join(toolRoot, "bin", "wasm-bindgen");
}
if (bindingVersion(bindgen) !== `wasm-bindgen ${version}`) {
  throw new Error(`Requires wasm-bindgen ${version}. Set WASM_BINDGEN to that binary, ` +
    "or run npm run build:scene-wasm -- --install-tools for a project-local install.");
}

run("cargo", ["build", "--locked", "--release", "-p", "opencanvas-scene-wasm",
  "--target", "wasm32-unknown-unknown", "--target-dir", targetDir], {
  env: { ...process.env, CARGO_PROFILE_RELEASE_OPT_LEVEL: "z",
    CARGO_PROFILE_RELEASE_LTO: "true", CARGO_PROFILE_RELEASE_CODEGEN_UNITS: "1",
    CARGO_PROFILE_RELEASE_PANIC: "abort" },
});
mkdirSync(output, { recursive: true });
run(bindgen, [join(targetDir, "wasm32-unknown-unknown", "release", "opencanvas_scene_wasm.wasm"),
  "--target", "web", "--out-dir", output, "--out-name", "opencanvas_scene_wasm"]);
// Vite serves/copies these as ESM from public/, not as an npm CommonJS package.
rmSync(join(output, "package.json"), { force: true });
for (const name of ["opencanvas_scene_wasm.js", "opencanvas_scene_wasm_bg.wasm"]) {
  const bytes = readFileSync(join(output, name));
  const gzip = gzipSync(bytes, { level: 9 });
  const brotli = brotliCompressSync(bytes, { params: {
    [constants.BROTLI_PARAM_QUALITY]: 11,
  } });
  console.log(`${name}: raw=${bytes.length} gzip=${gzip.length} brotli=${brotli.length} bytes`);
}
console.log(`ESM, WASM and generated declarations: ${output}`);
