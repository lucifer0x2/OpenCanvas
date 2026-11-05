import { readFileSync, writeFileSync } from "node:fs";
import { join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { brotliCompressSync, constants, gzipSync } from "node:zlib";

// Identical sidecar names and compression settings for local and Docker builds.
export function compressSceneWasm(output) {
  for (const name of [
    "opencanvas_scene_wasm.js",
    "opencanvas_scene_wasm_bg.wasm",
  ]) {
    const path = join(output, name);
    const bytes = readFileSync(path);
    const gzip = gzipSync(bytes, { level: 9 });
    const brotli = brotliCompressSync(bytes, {
      params: {
        [constants.BROTLI_PARAM_QUALITY]: 11,
      },
    });
    writeFileSync(`${path}.gz`, gzip);
    writeFileSync(`${path}.br`, brotli);
    console.log(
      `${name}: raw=${bytes.length} gzip=${gzip.length} brotli=${brotli.length} bytes`,
    );
  }
}

if (
  process.argv[1] &&
  resolve(process.argv[1]) === fileURLToPath(import.meta.url)
) {
  if (process.argv.length !== 3)
    throw new Error("Usage: node compress-scene-wasm.mjs <output-dir>");
  compressSceneWasm(resolve(process.argv[2]));
}
