import { node, scene, hash } from "./scene.mjs";
export function probes(resources) {
  const base = () => scene("probe", [node("shape", 0, 0, 10, 10)], resources);
  const cases = [], add = (name, value, code, scale = 1) => cases.push({ name, scene: value, scale, expectedChromeCode: code });
  const version = base(); version.provenance.rendererVersion = "wrong";
  add("pipeline-version", version, "PIPELINE_MISMATCH");
  add("scale-zero", base(), "PIXEL_BUDGET", 0);
  const budget = base(); budget.provenance.viewport = { width: 5000, height: 5000 };
  add("target-budget", budget, "PIXEL_BUDGET");
  for (const [name, url, digest] of [
    ["missing-image", "/resources/missing.png", "0".repeat(64)],
    ["hash-mismatch-image", "/resources/png.bin", "0".repeat(64)],
    ["decode-corrupt-image", "/resources/broken.png", hash("not an image")],
  ]) {
    const value = base(); value.nodes[0].kind = "image"; value.nodes[0].assetId = "broken";
    value.assets = [{ id: "broken", url, hash: digest, mediaType: "image/png" }];
    add(name, value, "RESOURCE_NOT_READY");
  }
  const font = base(); font.nodes[0] = node("text", 0, 0, 10, 10, { kind: "text", fontId: "broken", lines: [{ text: "X", x: 0, y: 10, width: 10 }] });
  font.fonts = [{ id: "broken", url: "/resources/broken.ttf", hash: hash("not a font"), family: "Broken", weight: 400, style: "normal" }];
  add("decode-corrupt-font", font, "RESOURCE_NOT_READY");
  return cases;
}
