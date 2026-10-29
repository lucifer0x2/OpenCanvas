/**
 * Builds absolutely positioned DOM for a computed scene. The browser never lays out or wraps:
 * bounds, text lines (baseline positions, as SVG text) and image placement come from the scene
 * and the shared renderer helpers. Element per node:
 *   .oc-node[data-oc-id]  position, size, opacity; animated translate/scale/rotate/opacity
 *     .oc-box             fill (background, or text colour), radius, clip; content + children
 *     .oc-stroke          border overlay after the subtree, outside the node's own clip
 */
import { placeImage } from "../../../renderer/image-fit";
import type { Resources, Scene, SceneNode } from "../../../renderer/types";
import {
  boxDeclarations,
  nodeDeclarations,
  strokeDeclarations,
  type Declarations,
} from "./paintStyle";

export interface NodeElements {
  node: HTMLElement;
  box: HTMLElement;
}

export interface SceneDom {
  root: HTMLElement;
  elements: Map<string, NodeElements>;
}

const SVG = "http://www.w3.org/2000/svg";

function apply(element: HTMLElement | SVGElement, declarations: Declarations) {
  for (const [key, value] of Object.entries(declarations))
    element.style.setProperty(key, value);
}

function placeholder(box: HTMLElement, label: string, background?: string) {
  const note = document.createElement("span");
  note.className = "oc-missing";
  note.textContent = label;
  if (background) box.style.background = background;
  box.append(note);
}

function textContent(box: HTMLElement, node: SceneNode, resources: Resources) {
  const font = resources.fonts.get(node.fontId ?? "");
  if (!font) return placeholder(box, "[Missing font]");
  const svg = document.createElementNS(SVG, "svg");
  svg.setAttribute("class", "oc-text");
  svg.setAttribute("width", String(node.bounds.width));
  svg.setAttribute("height", String(node.bounds.height));
  apply(svg, {
    "font-family": `"${font.family}"`,
    "font-weight": String(font.weight),
    "font-style": font.style,
    "font-size": `${node.fontSize}px`,
  });
  for (const line of node.lines) {
    const text = document.createElementNS(SVG, "text");
    text.setAttribute("x", String(line.x));
    text.setAttribute("y", String(line.y));
    text.textContent = line.text;
    svg.append(text);
  }
  box.append(svg);
}

function imageContent(
  box: HTMLElement,
  node: SceneNode,
  scene: Scene,
  resources: Resources,
) {
  const decoded = resources.images.get(node.assetId ?? "");
  const asset = scene.assets.find((entry) => entry.id === node.assetId);
  if (!decoded || !asset) return placeholder(box, "[Missing image]", "#ffe4e6");
  const crop = node.crop ?? { x: 0, y: 0, width: 1, height: 1 };
  const source = {
    width: decoded.naturalWidth * crop.width,
    height: decoded.naturalHeight * crop.height,
  };
  const at = placeImage(node.fit, node.bounds, source);
  const fullWidth = at.width / crop.width;
  const fullHeight = at.height / crop.height;
  const image = document.createElement("img");
  image.className = "oc-image";
  image.alt = "";
  image.draggable = false;
  image.src = asset.url;
  apply(image, {
    left: `${at.x - crop.x * fullWidth}px`,
    top: `${at.y - crop.y * fullHeight}px`,
    width: `${fullWidth}px`,
    height: `${fullHeight}px`,
  });
  box.append(image);
}

export function buildSceneDom(
  scene: Scene,
  resources: Resources,
  strokeIds: Set<string>,
): SceneDom {
  const root = document.createElement("div");
  root.className = "oc-scene";
  root.style.width = `${scene.provenance.viewport.width}px`;
  root.style.height = `${scene.provenance.viewport.height}px`;
  const ids = new Set(scene.nodes.map((node) => node.id));
  const byId = new Map(scene.nodes.map((node) => [node.id, node]));
  const elements = new Map<string, NodeElements>();
  // Scene order is paint order; a parent always precedes its children.
  for (const node of scene.nodes) {
    const parent =
      node.parentId && ids.has(node.parentId) ? byId.get(node.parentId)! : null;
    const element = document.createElement("div");
    element.className = `oc-node oc-kind-${node.kind}`;
    element.dataset.ocId = node.id;
    apply(element, {
      left: `${node.bounds.x - (parent?.bounds.x ?? 0)}px`,
      top: `${node.bounds.y - (parent?.bounds.y ?? 0)}px`,
      width: `${node.bounds.width}px`,
      height: `${node.bounds.height}px`,
      ...nodeDeclarations(node),
    });
    const box = document.createElement("div");
    box.className = "oc-box";
    apply(box, boxDeclarations(node));
    // An image always clips to its rounded bounds, like renderer/paint.ts.
    if (node.clip || node.kind === "image") box.style.overflow = "hidden";
    if (node.kind === "text") textContent(box, node, resources);
    if (node.kind === "image") imageContent(box, node, scene, resources);
    element.append(box);
    if (strokeIds.has(node.id)) {
      const stroke = document.createElement("div");
      stroke.className = "oc-stroke";
      apply(stroke, strokeDeclarations(node.stroke, node.radius ?? 0));
      element.append(stroke);
    }
    (parent ? elements.get(parent.id)!.box : root).append(element);
    elements.set(node.id, { node: element, box });
  }
  return { root, elements };
}
