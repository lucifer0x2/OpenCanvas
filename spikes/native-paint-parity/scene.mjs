import { createHash } from "node:crypto";
export const hash = bytes => createHash("sha256").update(bytes).digest("hex");
export function node(id, x, y, width, height, props = {}) {
  return { id, parentId: null, kind: "rectangle", bounds: { x, y, width, height },
    lines: [], fontId: null, fontSize: 24, lineHeight: 30, fill: "#4682b4", assetId: null,
    fit: "cover", opacity: 1, clip: false, radius: 0, crop: null, ...props };
}
export function scene(name, nodes, resources, width = 240, height = 160) {
  const fonts = resources.fonts.filter(f => nodes.some(n => n.fontId === f.id));
  const assets = resources.assets.filter(a => nodes.some(n => n.assetId === a.id));
  return { schemaVersion: 1, coordinateSpace: "frame-local", provenance: {
    projectId: "parity-fixture", revision: 1, frameId: name, viewport: { width, height },
    theme: "default", state: "default", sceneVersion: "scene-3", rendererVersion: "web-4",
    sourceSceneKey: hash(name), fontManifestHash: hash(JSON.stringify(fonts)), assetManifestHash: hash(JSON.stringify(assets)),
  }, nodes, fonts, assets, diagnostics: [] };
}
export const stroke = (align = "inside", width = [5, 9, 3, 7]) => ({ align, width,
  color: ["#ef4444", "#22c55e99", "#3b82f6", "#a855f780"] });
export const shadow = (inset, spread = 3, blur = 8) => ({ x: 5, y: 6, blur, spread, color: "#15273899", inset });
