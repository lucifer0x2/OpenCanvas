/** Paint cached text without measuring, shaping fragments independently or wrapping. */
import { fontCSS } from "./font-family";
import type { Resources, SceneNode } from "./types";
export function paintText(context: CanvasRenderingContext2D, node: SceneNode, resources: Resources) {
  const b = node.bounds, font = resources.fonts.get(node.fontId ?? "");
  if (!font) {
    context.fillStyle = "#be123c"; context.font = "12px sans-serif";
    context.fillText("[Missing font]", b.x, b.y + 14); return;
  }
  context.font = fontCSS(font, node.fontSize);
  context.fontKerning = "normal"; context.letterSpacing = `${node.letterSpacing ?? 0}px`;
  context.textBaseline = "alphabetic";
  context.fillStyle = typeof node.fill === "string" ? node.fill : "transparent";
  for (const line of node.lines) {
    context.fillText(line.text, b.x + line.x, b.y + line.y);
    const d = node.decoration;
    if (d) context.fillRect(b.x + line.x, Math.round(b.y + line.y + d.offset), line.width, d.thickness);
  }
}
