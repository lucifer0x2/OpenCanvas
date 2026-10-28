import {
  MAX_PIXELS,
  type Resources,
  type Scene,
  type SceneNode,
} from "./types";
import { indexScene, intersect, rectOf, type LogicalRect } from "./scene-tree";

/** Pixels all live opacity layers may hold at once; beyond it a group folds its alpha instead. */
export const MAX_LAYER_PIXELS = MAX_PIXELS;

export type PaintNode = (
  context: CanvasRenderingContext2D,
  node: SceneNode,
  resources: Resources,
) => void;

export interface PaintStats {
  /** Groups composited through an offscreen layer. */
  opacityLayers: number;
  /** Groups that needed a layer but folded their alpha into children because of the budget. */
  opacityLayerFallbacks: number;
}

/** The device-pixel box of a scene-unit rect under `m`, grown to whole pixels plus one for AA. */
function toDevice(rect: LogicalRect, m: DOMMatrix, clip: LogicalRect) {
  if (!Number.isFinite(rect.x0 + rect.y0 + rect.x1 + rect.y1)) return clip;
  const corners = [rect.x0, rect.x1].flatMap((x) =>
    [rect.y0, rect.y1].map((y) => [
      x * m.a + y * m.c + m.e,
      x * m.b + y * m.d + m.f,
    ]),
  );
  const px = corners.map((c) => c[0]),
    py = corners.map((c) => c[1]);
  return intersect(
    {
      x0: Math.floor(Math.min(...px)) - 1,
      y0: Math.floor(Math.min(...py)) - 1,
      x1: Math.ceil(Math.max(...px)) + 1,
      y1: Math.ceil(Math.max(...py)) + 1,
    },
    clip,
  );
}

/**
 * Paints the scene in tree order under the context's current transform, with CSS group opacity:
 * a node below opacity 1 whose subtree issues two or more paints renders into a layer at alpha 1,
 * composited once at its accumulated opacity. Otherwise its alpha folds into its own paint and its
 * children, which is identical for a single paint. Layers are HTMLCanvasElements because
 * OffscreenCanvas clips without antialiasing in Chromium.
 */
export function paintTree(
  context: CanvasRenderingContext2D,
  scene: Scene,
  resources: Resources,
  paintNode: PaintNode,
  options: { maxLayerPixels?: number } = {},
): PaintStats {
  const tree = indexScene(scene, resources);
  const budget = options.maxLayerPixels ?? MAX_LAYER_PIXELS;
  const stats: PaintStats = { opacityLayers: 0, opacityLayerFallbacks: 0 };
  let livePixels = 0;

  // `clip` is the device-pixel box that can still show paint on `ctx`'s canvas.
  const draw = (
    ctx: CanvasRenderingContext2D,
    node: SceneNode,
    alpha: number,
    clip: LogicalRect,
  ) => {
    const a = alpha * node.opacity;
    if (!(a > 0)) return;
    if (a < 1 && (tree.units.get(node.id) ?? 0) >= 2) {
      const extent = tree.extent.get(node.id);
      const r = extent && toDevice(extent, ctx.getTransform(), clip);
      if (!r) return;
      const w = r.x1 - r.x0,
        h = r.y1 - r.y0;
      if (livePixels + w * h <= budget) {
        composite(ctx, node, a, r, w, h);
        return;
      }
      stats.opacityLayerFallbacks++;
    }
    contents(ctx, node, a, clip);
  };

  const composite = (
    ctx: CanvasRenderingContext2D,
    node: SceneNode,
    alpha: number,
    r: LogicalRect,
    w: number,
    h: number,
  ) => {
    const layer = document.createElement("canvas");
    layer.width = w;
    layer.height = h;
    const lctx = layer.getContext("2d");
    if (!lctx) return contents(ctx, node, alpha, r);
    const m = ctx.getTransform();
    // Whole-pixel offset only, so antialiasing lands on the same grid as direct painting.
    lctx.setTransform(m.a, m.b, m.c, m.d, m.e - r.x0, m.f - r.y0);
    lctx.imageSmoothingEnabled = ctx.imageSmoothingEnabled;
    lctx.imageSmoothingQuality = ctx.imageSmoothingQuality;
    stats.opacityLayers++;
    livePixels += w * h;
    contents(lctx, node, 1, { x0: 0, y0: 0, x1: w, y1: h });
    livePixels -= w * h;
    ctx.save();
    // setTransform keeps the clip path, so clipping ancestors still bound the composite.
    ctx.setTransform(1, 0, 0, 1, 0, 0);
    ctx.globalAlpha = alpha;
    ctx.imageSmoothingEnabled = false;
    ctx.drawImage(layer, r.x0, r.y0);
    ctx.restore();
    layer.width = layer.height = 0;
  };

  const contents = (
    ctx: CanvasRenderingContext2D,
    node: SceneNode,
    alpha: number,
    clip: LogicalRect,
  ) => {
    ctx.save();
    ctx.globalAlpha = alpha;
    paintNode(ctx, node, resources);
    const children = tree.children.get(node.id);
    if (children) {
      let inner: LogicalRect | null = clip;
      if (node.clip) {
        const b = node.bounds;
        inner = toDevice(rectOf(b), ctx.getTransform(), clip);
        ctx.beginPath();
        ctx.roundRect(b.x, b.y, b.width, b.height, node.radius ?? 0);
        ctx.clip();
      }
      if (inner) for (const child of children) draw(ctx, child, alpha, inner);
    }
    ctx.restore();
  };

  const canvas = {
    x0: 0,
    y0: 0,
    x1: context.canvas.width,
    y1: context.canvas.height,
  };
  for (const root of tree.roots) draw(context, root, 1, canvas);
  return stats;
}
