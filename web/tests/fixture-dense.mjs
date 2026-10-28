import { createHash } from "node:crypto";
import { dashboard, node } from "./fixture-dashboard.mjs";
import { stripesPNG } from "./fixture-png.mjs";

// Zoom fixtures heavier than the 800x600 viewer card: "dense" is a 1440x900 dashboard (215
// nodes, 179 text lines, one striped image), "stress" tiles it 2x2 into 2880x1800.

export function denseFixtureData(hash, kind) {
  const tiles = kind === "stress" ? 2 : 1;
  const width = 1440 * tiles,
    height = 900 * tiles;
  const png = stripesPNG();
  const pngHash = createHash("sha256").update(png).digest("hex");
  const children = [];
  for (let ty = 0; ty < tiles; ty++)
    for (let tx = 0; tx < tiles; tx++)
      children.push(
        ...dashboard(tiles > 1 ? `t${tx}${ty}-` : "", tx * 1440, ty * 900),
      );
  const page = {
    id: "page",
    parentId: null,
    kind: "page",
    sortKey: 0,
    props: { name: "Website" },
  };
  const frame = {
    id: "frame",
    parentId: "page",
    kind: "frame",
    sortKey: 0,
    props: { name: "Dashboard", width, height, fill: "#ffffff" },
  };
  const scene = (rev) => ({
    schemaVersion: 1,
    coordinateSpace: "frame-local",
    diagnostics: [],
    provenance: {
      projectId: "fixture",
      revision: rev,
      frameId: "frame",
      viewport: { width, height },
      theme: "default",
      state: "default",
      sceneVersion: "scene-1",
      rendererVersion: "web-1",
      sourceSceneKey: `${kind}-${rev}`,
      fontManifestHash: hash,
      assetManifestHash: pngHash,
    },
    fonts: [
      {
        id: "font_bundled",
        family: "OpenCanvas Noto Sans",
        hash,
        url: "/api/v1/font",
        weight: 400,
        style: "normal",
      },
    ],
    assets: [
      {
        id: "stripes",
        hash: pngHash,
        url: "/api/v1/asset",
        mediaType: "image/png",
      },
    ],
    nodes: [
      node("frame", "frame", { x: 0, y: 0, width, height }, "#fff"),
      ...children,
    ],
  });
  return { nodes: () => [page, frame], scene, asset: png };
}
