// Build only this isolated workspace; protect production and borrowed harness identities.
import { execFile } from "node:child_process";
import { promisify } from "node:util";
import { readFile, writeFile, readdir, mkdir, copyFile } from "node:fs/promises";
import { join, relative } from "node:path";
import { sourceHashes } from "../native-paint-parity/runtime.mjs";
import { hash } from "../native-paint-parity/scene.mjs";
const exec = promisify(execFile);
export async function identities(root, here) {
  const result = await sourceHashes(root);
  async function walk(dir) {
    for (const entry of await readdir(dir, { withFileTypes: true })) {
      if (["out", "target", "node_modules", ".git"].includes(entry.name)) continue;
      const path = join(dir, entry.name);
      if (entry.isDirectory()) await walk(path);
      else if (/\.(rs|mjs|toml|md)$/.test(entry.name)) result[relative(root, path)] = hash(await readFile(path));
    }
  }
  await walk(here);
  await walk(join(root, "crates/layout/src"));
  for (const name of ["Cargo.toml", "Cargo.lock", "crates/render/Cargo.toml", "crates/layout/Cargo.toml", "crates/text/Cargo.toml"])
    result[name] = hash(await readFile(join(root, name)));
  return Object.fromEntries(Object.entries(result).sort(([a], [b]) => a.localeCompare(b)));
}
export async function build(here, out) {
  const env = { ...process.env, CARGO_TARGET_DIR: join(here, "out/target") };
  const args = ["--manifest-path", join(here, "Cargo.toml"), "--offline"];
  // Seed the isolated ignored lock from the product's pins, not latest cached transitives.
  await copyFile(new URL("../../Cargo.lock", import.meta.url), join(here, "Cargo.lock"));
  try {
    const run = await exec("cargo", ["build", ...args, "--bins"], { env, maxBuffer: 8 << 20 });
    await writeFile(join(out, "build.log"), run.stdout + run.stderr);
  } catch (error) {
    await writeFile(join(out, "build.log"), (error.stdout ?? "") + (error.stderr ?? "")); throw error;
  }
  await mkdir(join(out, "bin"));
  for (const name of ["icc-fixtures", "icc-native-adapter"])
    await copyFile(join(env.CARGO_TARGET_DIR, "debug", name), join(out, "bin", name));
  await exec(join(out, "bin/icc-fixtures"), [out], { maxBuffer: 8 << 20 });
  const metadata = JSON.parse((await exec("cargo", ["metadata", ...args, "--format-version", "1"], { env, maxBuffer: 16 << 20 })).stdout);
  const lock = await readFile(join(here, "Cargo.lock"));
  await writeFile(join(out, "Cargo.lock"), lock);
  const deps = metadata.packages.map(p => ({ name: p.name, version: p.version, license: p.license, source: p.source }));
  // Every registry version must already exist in the product lock; no new versions.
  const productLock = await readFile(new URL("../../Cargo.lock", import.meta.url), "utf8");
  for (const p of deps.filter(p => p.source)) {
    if (!productLock.includes(`name = "${p.name}"\nversion = "${p.version}"`)) throw Error(`Unapproved dependency ${p.name}@${p.version}`);
  }
  return { binary: join(out, "bin/icc-native-adapter"), deps, lockHash: hash(lock),
    rustc: (await exec("rustc", ["--version"])).stdout.trim(),
    binaryHash: hash(await readFile(join(out, "bin/icc-native-adapter"))),
    generatorHash: hash(await readFile(join(out, "bin/icc-fixtures"))) };
}

export function changed(before, after) {
  return [...new Set([...Object.keys(before), ...Object.keys(after)])].filter(k => before[k] !== after[k]);
}

export async function browserIdentity(browser) {
  const cdp = await browser.newBrowserCDPSession();
  try {
    const runtime = await cdp.send("Browser.getVersion");
    // getBrowserCommandLine is disabled in this pinned headless build. CDP owns
    // the PID; inspect only this newly spawned browser, never enumerate host apps.
    const { processInfo } = await cdp.send("SystemInfo.getProcessInfo");
    const process = processInfo.find(p => p.type === "browser");
    if (!process || !Number.isInteger(process.id) || process.id <= 0) throw Error("Browser PID unavailable");
    const actualExecutable = (await exec("ps", ["-ww", "-p", String(process.id), "-o", "comm="])).stdout.trim();
    const launchCommandLine = (await exec("ps", ["-ww", "-p", String(process.id), "-o", "args="])).stdout.trim();
    return { actualExecutable, executableHash: hash(await readFile(actualExecutable)), launchCommandLine, runtime };
  } finally { await cdp.detach(); }
}
