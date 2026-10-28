import { createHash } from "node:crypto";
import { BUNDLED_FONT } from "../../worker/pipeline.mjs";

export const imageSVG =
  '<svg xmlns="http://www.w3.org/2000/svg" width="80" height="40"><path fill="#2563eb" d="M0 0h40v40H0z"/><path fill="#f59e0b" d="M40 0h40v40H40z"/></svg>';
export const imageHash = createHash("sha256").update(imageSVG).digest("hex");
export const node = (id, parentId, kind, props) => ({
  id,
  parentId,
  kind,
  props,
});
export function fixture(width = 420) {
  return {
    schemaVersion: 1,
    projectId: "fixture",
    revision: 7,
    frameId: "frame",
    viewport: { width, height: 600 },
    theme: "default",
    state: "default",
    fonts: [{ ...BUNDLED_FONT }],
    assets: [
      {
        id: "image",
        hash: imageHash,
        url: "/fixture.svg",
        mediaType: "image/svg+xml",
      },
    ],
    nodes: [
      node("frame", null, "frame", {
        width,
        height: 600,
        fill: "#ffffff",
        layout: { mode: "column", padding: 20, gap: 12 },
      }),
      node("title", "frame", "text", {
        text: "Thiết kế tiếng Việt: Trường học và những điều mới mẻ",
        fontId: "font_bundled",
        fontSize: 24,
        lineHeight: 32,
        width: "fill",
        height: "hug",
        fill: "#10243a",
      }),
      node("row", "frame", "group", {
        width: "fill",
        height: "hug",
        layout: { mode: "row", padding: 8, gap: 12 },
        fill: "#f1f5f9",
      }),
      node("box", "row", "rectangle", {
        width: 40,
        height: 40,
        fill: "#ef4444",
        radius: 6,
      }),
      node("body", "row", "text", {
        text: "Cùng xây dựng một không gian sáng tạo. Tiếng Việt có dấu: ă â đ ê ô ơ ư.",
        fontId: "font_bundled",
        fontSize: 16,
        lineHeight: 24,
        width: "fill",
        height: "hug",
        fill: "#334155",
      }),
      node("picture", "frame", "image", {
        width: 160,
        height: 80,
        assetId: "image",
        fit: "contain",
      }),
    ],
  };
}
