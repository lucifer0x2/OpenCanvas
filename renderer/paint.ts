import type { FontManifest, Resources, Scene, SceneNode } from "./types";
import { PipelineError } from "./error";
import { placeImage } from "./image-fit";

const fontCSS = (font: FontManifest, size: number) =>
  `${font.style} ${font.weight} ${size}px "${font.family}"`;

function paintFill(context: CanvasRenderingContext2D, node: SceneNode) {
  if (node.fill === "transparent" || node.kind === "text") return;
  const b = node.bounds;
  context.fillStyle = node.fill;
  context.beginPath();
  context.roundRect(b.x, b.y, b.width, b.height, node.radius ?? 0);
  context.fill();
}

function paintText(
  context: CanvasRenderingContext2D,
  node: SceneNode,
  resources: Resources,
) {
  const b = node.bounds;
  const font = resources.fonts.get(node.fontId ?? "");
  if (!font) {
    context.fillStyle = "#be123c";
    context.font = "12px sans-serif";
    context.fillText("[Missing font]", b.x, b.y + 14);
    return;
  }
  context.font = fontCSS(font, node.fontSize);
  context.fontKerning = "normal";
  context.textBaseline = "alphabetic";
  context.fillStyle = node.fill;
  // Do not call measureText, wrap, use maxWidth, or Konva.Text here.
  for (const line of node.lines)
    context.fillText(line.text, b.x + line.x, b.y + line.y);
}

function paintImage(
  context: CanvasRenderingContext2D,
  node: SceneNode,
  resources: Resources,
) {
  const b = node.bounds;
  const image = resources.images.get(node.assetId ?? "");
  if (!image) {
    context.fillStyle = "#ffe4e6";
    context.fillRect(b.x, b.y, b.width, b.height);
    context.fillStyle = "#9f1239";
    context.font = "12px sans-serif";
    context.fillText("[Missing image]", b.x + 4, b.y + 16);
    return;
  }
  context.save();
  context.beginPath();
  context.roundRect(b.x, b.y, b.width, b.height, node.radius ?? 0);
  context.clip();
  const crop = node.crop ?? { x: 0, y: 0, width: 1, height: 1 };
  const sw = image.naturalWidth * crop.width,
    sh = image.naturalHeight * crop.height;
  const at = placeImage(node.fit, b, { width: sw, height: sh });
  context.drawImage(
    image,
    image.naturalWidth * crop.x,
    image.naturalHeight * crop.y,
    sw,
    sh,
    b.x + at.x,
    b.y + at.y,
    at.width,
    at.height,
  );
  context.restore();
}

/** One node's own paint, in scene units. Order: fill, then text or image content. */
function paintNode(
  context: CanvasRenderingContext2D,
  node: SceneNode,
  resources: Resources,
) {
  paintFill(context, node);
  if (node.kind === "text") paintText(context, node, resources);
  if (node.kind === "image") paintImage(context, node, resources);
}

/** Clips to every clipping ancestor and returns the node's effective opacity. */
function enterAncestors(
  context: CanvasRenderingContext2D,
  node: SceneNode,
  byId: Map<string, SceneNode>,
  limit: number,
): number {
  let opacity = node.opacity;
  let parent = node.parentId ? byId.get(node.parentId) : undefined;
  let guard = 0;
  while (parent) {
    if (++guard > limit)
      throw new PipelineError("SCENE_INVALID", "Scene cycle");
    opacity *= parent.opacity;
    if (parent.clip) {
      const b = parent.bounds;
      context.beginPath();
      context.roundRect(b.x, b.y, b.width, b.height, parent.radius ?? 0);
      context.clip();
    }
    parent = parent.parentId ? byId.get(parent.parentId) : undefined;
  }
  return opacity;
}

/**
 * Paints the scene in scene units under the context's current transform. Never resizes or clears
 * the target, so the worker PNG (via drawScene) and the viewer's zoomed Konva layer share it.
 */
export function paintScene(
  context: CanvasRenderingContext2D,
  scene: Scene,
  resources: Resources,
): void {
  const byId = new Map(scene.nodes.map((n) => [n.id, n]));
  for (const node of scene.nodes) {
    context.save();
    context.globalAlpha = enterAncestors(
      context,
      node,
      byId,
      scene.nodes.length,
    );
    paintNode(context, node, resources);
    context.restore();
  }
}
