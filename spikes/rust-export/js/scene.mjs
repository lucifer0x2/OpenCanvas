// Builds data/scene.json: a hand-built 400×300 Scene (renderer/types.ts shape) plus diff regions.
import { readFileSync, writeFileSync } from "node:fs";
import { createHash } from "node:crypto";
const dir = new URL("../data/", import.meta.url);
const assetHash = createHash("sha256").update(readFileSync(new URL("asset.png", dir))).digest("hex");
export const FONT = { id: "font_bundled", family: "OpenCanvas Noto Sans",
  hash: "bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d",
  url: "/renderer/fonts/NotoSans.ttf", weight: 400, style: "normal" };
export const ASSET = { id: "asset_img", hash: assetHash, url: "/asset.png", mediaType: "image/png" };
const r64 = (v) => Math.round(v * 64) / 64;
const base = { lines: [], fontId: null, fontSize: 0, lineHeight: 0, fill: "transparent", assetId: null,
  fit: "fit", opacity: 1, clip: false, radius: 0, crop: null };
const box = (id, parentId, x, y, width, height, extra = {}) =>
  ({ ...base, id, parentId, kind: "frame", bounds: { x, y, width, height }, ...extra });
const text = (id, parentId, x, y, width, size, lh, lines, fill) => ({
  ...base, id, parentId, kind: "text", fontId: FONT.id, fontSize: size, lineHeight: lh, fill,
  bounds: { x, y, width, height: lines.length * lh },
  lines: lines.map((t, n) => ({ text: t, x: 0, y: r64(n * lh + size + (lh - size) / 2), width }))});
const stroke = (color, w) => ({ color: [color, color, color, color], width: [w, w, w, w], align: "inside" });
const nodes = [
  box("root", null, 0, 0, 400, 300, { fill: "#ffffff" }),
  box("card", "root", 16, 16, 368, 150, { fill: "#eef2ff", radius: 12, stroke: stroke("#6366f1", 1) }),
  text("t1", "card", 28, 26, 260, 14, 20, ["Đăng nhập tài khoản của bạn", "Tiếng Việt: ắ ằ ẳ ẵ ặ ể ễ ộ ợ ữ ỹ"], "#111827"),
  text("t2", "card", 28, 70, 260, 24, 32, ["Export AV To Ty We"], "#1e3a8a"),
  text("t3", "card", 28, 106, 260, 24, 32, ["Thiết kế đẹp, Ưu tiên"], "#0f172a"),
  { ...base, id: "img", parentId: "card", kind: "image", bounds: { x: 296, y: 28, width: 76, height: 60 },
    assetId: ASSET.id, fit: "cover", radius: 10 },
  box("clipParent", "root", 16, 180, 180, 104, { fill: "#f1f5f9", radius: 16, clip: true, stroke: stroke("#0f172a", 2) }),
  box("clipA", "clipParent", 96, 220, 140, 90, { fill: "#f97316" }),
  box("clipB", "clipParent", 4, 170, 60, 50, { fill: "#10b981" }),
  box("group", "root", 212, 180, 172, 104, { opacity: 0.5 }),
  box("gA", "group", 222, 190, 96, 60, { fill: "#2563eb" }),
  box("gB", "group", 270, 220, 100, 56, { fill: "#dc2626", radius: 8 }),
];
const scene = {
  schemaVersion: 1, coordinateSpace: "frame",
  provenance: { projectId: "spike", revision: 1, frameId: "root", viewport: { width: 400, height: 300 },
    theme: "light", state: "default", sceneVersion: "scene-2", rendererVersion: "web-2",
    sourceSceneKey: "spike", fontManifestHash: "x", assetManifestHash: "x" },
  nodes, fonts: [FONT], assets: [ASSET], diagnostics: [],
  // Diff regions in scene units (first match wins; rest = shapes).
  regions: [{ label: "text", x: 24, y: 20, w: 268, h: 130 }, { label: "image", x: 296, y: 28, w: 76, h: 60 }],
};
writeFileSync(new URL("scene.json", dir), JSON.stringify(scene, null, 1));
console.log("scene.json written, asset", assetHash.slice(0, 12));
