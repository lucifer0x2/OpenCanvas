import { createHash } from "node:crypto";
import { readFile, access } from "node:fs/promises";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

export const ROOT = fileURLToPath(new URL("../../", import.meta.url));
export const OUT = new URL("./out/", import.meta.url);
export const sha256 = (bytes) => createHash("sha256").update(bytes).digest("hex");
export const json = async (path) => JSON.parse(await readFile(path, "utf8"));
export const PIN = { playwright: "1.64.1", chromium: "156.0.8078.4", revision: "1248" };

export async function prepareRenderer() {
  try {
    await access(new URL("../../renderer/dist/index.js", import.meta.url));
  } catch {
    const result = spawnSync("npm", ["run", "build:renderer"], { cwd: ROOT, stdio: "inherit" });
    if (result.error || result.status !== 0) throw result.error ?? new Error("Renderer build failed");
  }
  const files = ["index.ts", "measurement.ts", "text-wrap.ts", "text-transform.ts",
    "resources.ts", "font-family.ts", "glyph-coverage.ts", "types.ts", "dist/index.js"];
  const hashes = {};
  for (const file of files) hashes[`renderer/${file}`] = sha256(await readFile(`${ROOT}renderer/${file}`));
  const { build } = await import("esbuild");
  const current = await build({ absWorkingDir: ROOT, entryPoints: ["renderer/index.ts"],
    bundle: true, format: "esm", write: false });
  if (sha256(current.outputFiles[0].contents) !== hashes["renderer/dist/index.js"])
    throw new Error("Renderer bundle is stale: run npm run build:renderer before capture");
  for (const file of ["capture", "environment", "font-fixtures", "corpus", "browser-capture", "http-server", "schema"])
    hashes[`spikes/native-render-parity/${file}.mjs`] = sha256(await readFile(new URL(`${file}.mjs`, import.meta.url)));
  // Diagnostics call the actual current CSS transform helper, not a JS casing approximation.
  await build({ entryPoints: [`${ROOT}renderer/text-transform.ts`], bundle: true, format: "esm",
    outfile: fileURLToPath(new URL("transform.js", OUT)) });
  return hashes;
}

export async function playwrightRuntime() {
  process.env.PLAYWRIGHT_BROWSERS_PATH ??= `${ROOT}.runtime/browsers`;
  const pkg = await json(`${ROOT}node_modules/playwright/package.json`);
  const installed = await json(`${ROOT}node_modules/playwright-core/browsers.json`);
  const chrome = installed.browsers.find((b) => b.name === "chromium");
  if (pkg.version !== PIN.playwright || chrome?.browserVersion !== PIN.chromium || chrome?.revision !== PIN.revision)
    throw new Error(`Runtime pin mismatch: expected ${JSON.stringify(PIN)}`);
  return import("playwright");
}
