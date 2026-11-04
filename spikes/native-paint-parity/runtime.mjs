import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { readdir, readFile, writeFile } from "node:fs/promises";
import { join, relative } from "node:path";
import { hash } from "./scene.mjs";
const exec = promisify(execFile);
export async function sourceHashes(root) {
  const result = {};
  async function walk(dir) {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      if (["target", "out", "node_modules", ".git", "dist"].includes(entry.name)) continue;
      const path = join(dir, entry.name);
      if (entry.isDirectory()) await walk(path);
      else if (/\.(rs|mjs|ts|toml)$/.test(entry.name)) result[relative(root, path)] = hash(await readFile(path));
    }
  }
  for (const dir of ["crates/render/src", "crates/text/src", "renderer", "spikes/native-paint-parity"]) {
    // Do not traverse outputs/dependencies. Only these source directories are visited.
    if (dir.endsWith("native-paint-parity")) {
      for (const entry of await readdir(join(root, dir))) if (/\.(mjs|md)$/.test(entry)) {
        result[`${dir}/${entry}`] = hash(await readFile(join(root, dir, entry)));
      }
      await walk(join(root, dir, "native"));
    } else if (dir === "renderer") {
      for (const entry of await readdir(join(root, dir))) if (entry.endsWith(".ts")) result[`${dir}/${entry}`] = hash(await readFile(join(root, dir, entry)));
    } else await walk(join(root, dir));
  }
  return result;
}
export async function buildNative(root, out) {
  const manifest = join(root, "spikes/native-paint-parity/native/Cargo.toml");
  const env = { ...process.env, CARGO_TARGET_DIR: join(out, "target") };
  try {
    const build = await exec("cargo", ["build", "--manifest-path", manifest, "--offline"], { env, maxBuffer: 8 << 20 });
    await writeFile(join(out, "native-build.log"), build.stdout + build.stderr);
  } catch (error) {
    await writeFile(join(out, "native-build.log"), (error.stdout ?? "") + (error.stderr ?? ""));
    throw error;
  }
  const metadata = JSON.parse((await exec("cargo", ["metadata", "--manifest-path", manifest, "--offline", "--format-version", "1"], { env, maxBuffer: 16 << 20 })).stdout);
  const deps = metadata.packages.map(p => ({ name: p.name, version: p.version, license: p.license, source: p.source }));
  await writeFile(join(out, "native-Cargo.lock"), await readFile(join(root, "spikes/native-paint-parity/native/Cargo.lock")));
  return { binary: join(out, "target/debug/native-paint-parity-adapter"), deps,
    rustc: (await exec("rustc", ["--version"])).stdout.trim() };
}
export async function nativeCapture(binary, out, request, png) {
  const result = await exec(binary, [out, request, png], { maxBuffer: 2 << 20 });
  return JSON.parse(result.stdout);
}
