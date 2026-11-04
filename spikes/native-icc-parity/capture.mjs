import { readFile, writeFile, mkdir } from "node:fs/promises";
import { join } from "node:path";
import { capture } from "../native-paint-parity/oracle.mjs";
import { nativeCapture } from "../native-paint-parity/runtime.mjs";
import { scene, node, hash } from "../native-paint-parity/scene.mjs";
import { compare } from "../native-paint-parity/compare.mjs";
import { decodePNG, encodePNG } from "../native-paint-parity/png.mjs";
import { checkReference } from "./reference.mjs";
export const json = (file, data) => writeFile(file, JSON.stringify(data, null, 2) + "\n");

export async function one(page, native, out, f) {
  const bytes = await readFile(join(out, "resources", f.file));
  const asset = { id: f.id, mediaType: f.mediaType, hash: hash(bytes), url: `/resources/${f.file}` };
  // Verify loopback actually delivers the exact originals used by the native adapter.
  const browserHash = await page.evaluate(async url => {
    const data = await (await fetch(url)).arrayBuffer();
    return [...new Uint8Array(await crypto.subtle.digest("SHA-256", data))].map(v => v.toString(16).padStart(2, "0")).join("");
  }, asset.url);
  if (browserHash !== asset.hash) throw Error(`Original-byte mismatch ${f.id}`);
  const s = scene(f.id, [node("image", 0, 0, f.width, f.height, {
    kind: "image", fill: "transparent", fit: "fit", assetId: f.id,
  })], { fonts: [], assets: [asset] }, f.width, f.height);
  const dir = join(out, f.id); await mkdir(dir);
  const request = join(dir, "request.json"); await json(request, { scene: s, scale: 1 });
  const chrome = await capture(page, s, 1);
  const actual = await nativeCapture(native.binary, out, request, join(dir, "native.png"));
  const row = { ...f, source: { sha256: asset.hash, browserHash, bytes: bytes.length, license: "MIT (repository-authored fixture)",
    original: `resources/${f.file}` }, sceneHash: hash(JSON.stringify(s)), chrome: { ...chrome, png: undefined }, native: actual };
  const rasters = {};
  if (chrome.ok) {
    const png = Buffer.from(chrome.png, "base64"); await writeFile(join(dir, "chrome.png"), png);
    row.chrome.pngHash = hash(png); rasters.chrome = decodePNG(png);
  }
  if (actual.ok) {
    const png = await readFile(join(dir, "native.png")); row.native.pngHash = hash(png); rasters.native = decodePNG(png);
  }
  if (f.probe) {
    row.verdict = "ERROR_POLICY_PROBE_NOT_PARITY";
    row.nativeRejectsMalformedICC = !actual.ok && actual.code === "RESOURCE_NOT_READY";
    row.chromePolicy = chrome.ok ? "IGNORES_INVALID_PROFILE_AND_DECODES" : "REJECTS";
  } else if (chrome.ok && actual.ok) {
    const comparison = compare(rasters.chrome, rasters.native);
    row.comparison = comparison.report; row.parityVerdict = comparison.report.verdict;
    row.references = Object.fromEntries(Object.entries(rasters).map(([engine, raster]) => [engine, checkReference(f, raster)]));
    row.verdict = Object.values(row.references).some(r => r.checked && !r.pass) ? "ICC_REFERENCE_FAIL" : row.parityVerdict;
    row.drawStatsEqual = ["drawnLines", "opacityLayers", "opacityLayerFallbacks"].every(k => chrome.draw[k] === actual.draw[k]);
    await writeFile(join(dir, "diff.png"), encodePNG(comparison.diff));
    await writeFile(join(dir, "chrome-native-diff.png"), encodePNG(comparison.sideBySide));
  } else row.verdict = "DECODE_ERROR";
  await json(join(dir, "result.json"), row);
  return { row, rasters };
}
