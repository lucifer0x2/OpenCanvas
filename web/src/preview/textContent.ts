/** SVG text adapter: consumes the exact scene line origins and decoration geometry. */
import { fontFamily } from "../../../renderer/font-family";
import type { Resources, SceneNode } from "../../../renderer/types";
const SVG = "http://www.w3.org/2000/svg";

export function appendText(box: HTMLElement, node: SceneNode, resources: Resources): boolean {
  const font = resources.fonts.get(node.fontId ?? "");
  if (!font) return false;
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("class", "oc-text");
  svg.setAttribute("width", String(node.bounds.width));
  svg.setAttribute("height", String(node.bounds.height));
  Object.assign(svg.style, {
    fontFamily: `"${fontFamily(font)}"`, fontWeight: String(font.weight), fontStyle: font.style,
    fontSize: `${node.fontSize}px`, letterSpacing: `${node.letterSpacing ?? 0}px`,
    whiteSpace: "pre", fontKerning: "normal", fill: "currentColor", overflow: "visible",
  });
  for (const line of node.lines) {
    const text = document.createElementNS(SVG, "text");
    text.setAttribute("x", String(line.x)); text.setAttribute("y", String(line.y));
    text.setAttribute("xml:space", "preserve");
    text.textContent = line.text;
    svg.append(text);
    if (node.decoration) {
      const d = node.decoration;
      const rect = document.createElementNS(SVG, "rect");
      rect.setAttribute("x", String(line.x));
      rect.setAttribute("y", String(Math.round(node.bounds.y + line.y + d.offset) - node.bounds.y));
      rect.setAttribute("width", String(line.width)); rect.setAttribute("height", String(d.thickness));
      svg.append(rect);
    }
  }
  box.append(svg);
  return true;
}
