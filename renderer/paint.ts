import type { Resources, Scene, SceneNode } from "./types";
import { paintText } from "./paint-text";
import { paintTree, type PaintStats } from "./layers";
import { placeImage } from "./image-fit";
import { paintStroke } from "./stroke";
import { fillPath } from "./fill";
import { paintOuterShadows, paintInsetShadows } from "./shadow";

function paintFill(context: CanvasRenderingContext2D, node: SceneNode) {
  if (node.fill === "transparent" || node.kind === "text") return;
  const b = node.bounds;
  context.beginPath();
  context.roundRect(b.x, b.y, b.width, b.height, node.radius ?? 0);
  fillPath(context, node.fill, b);
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
export function paintNode(
  context: CanvasRenderingContext2D,
  node: SceneNode,
  resources: Resources,
) {
  paintOuterShadows(context, node);
  paintFill(context, node);
  if (node.kind === "text") paintText(context, node, resources);
  if (node.kind === "image") paintImage(context, node, resources);
  paintInsetShadows(context, node);
}

/**
 * Paints the scene in scene units under the context's current transform, with CSS group opacity
 * (see paintTree). Never resizes or clears the target, so the worker PNG (via drawScene) and the
 * viewer's zoomed Konva layer share it.
 */
export function paintScene(
  context: CanvasRenderingContext2D,
  scene: Scene,
  resources: Resources,
): PaintStats {
  // A stroke paints after the node's subtree, above its descendants: its own clip never cuts it;
  // ancestor clips and opacity (including its own group layer) do, as for its fill.
  return paintTree(context, scene, resources, paintNode, {
    paintOver: paintStroke,
  });
}
