import { BUNDLED_FONT } from "../../worker/pipeline.mjs";
import { node } from "./fixtures.mjs";

/** Every constraint has an observable exact bound; no dependency on system fonts. */
export function constraintsFixture(clip = true) {
  return {
    schemaVersion: 1,
    projectId: "constraints",
    revision: 3,
    frameId: "frame",
    viewport: { width: 360, height: 320 },
    theme: "default",
    state: "default",
    fonts: [{ ...BUNDLED_FONT }],
    assets: [],
    nodes: [
      node("frame", null, "frame", {
        width: 360,
        height: 320,
        fill: "#fff",
        layout: { mode: "column", padding: 10, gap: 8 },
      }),
      node("row", "frame", "group", {
        width: "fill",
        height: "hug",
        layout: { mode: "row", padding: 6, gap: 8 },
      }),
      node("fixed", "row", "rectangle", {
        width: 60,
        minWidth: 80,
        height: 50,
        maxHeight: 30,
        fill: "#2563eb",
      }),
      node("column", "row", "group", {
        width: "fill",
        maxWidth: 200,
        height: "hug",
        layout: { mode: "column", padding: 4, gap: 6 },
      }),
      node("text", "column", "text", {
        text: "Một\nHai",
        fontId: "font_bundled",
        fontSize: 16,
        lineHeight: 28,
        width: "fill",
        maxWidth: 150,
        height: "hug",
        minHeight: 70,
        fill: "#000",
      }),
      node("footer", "frame", "rectangle", {
        width: 20,
        minWidth: 40,
        height: 30,
        maxHeight: 18,
        fill: "#22c55e",
      }),
      node("overflow", "frame", "group", {
        width: 80,
        height: 30,
        clip,
        layout: { mode: "column" },
      }),
      node("overflow-child", "overflow", "rectangle", {
        width: 120,
        height: 30,
        fill: "#ef4444",
      }),
    ],
  };
}
