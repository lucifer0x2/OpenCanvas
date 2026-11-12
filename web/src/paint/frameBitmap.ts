import { union, type LogicalRect, type SceneTree } from "../../../renderer/index";
import type { Resources, Scene } from "../../../renderer/types";
import type { BoardPaint } from "./boardPaint";
import {
  bitmapBox,
  blitOrigin,
  placement,
  sameScale,
  type BitmapBox,
} from "./bitmapBox";

/** One ready frame's immutable paint input; a bitmap is valid only for these exact objects. */
export interface FrameContent {
  scene: Scene;
  resources: Resources;
  tree: SceneTree;
}
type Paint = (context: CanvasRenderingContext2D) => void;

const extents = new WeakMap<SceneTree, LogicalRect | null>();
/** Everything the scene paints, in scene units; null when unbounded (placeholders). */
export function sceneExtent(tree: SceneTree): LogicalRect | null {
  if (extents.has(tree)) return extents.get(tree) ?? null;
  let extent: LogicalRect | null = null;
  for (const root of tree.roots) extent = union(extent, tree.extent.get(root.id) ?? null);
  if (extent && !Number.isFinite(extent.x0 + extent.y0 + extent.x1 + extent.y1))
    extent = null;
  extents.set(tree, extent);
  return extent;
}

/**
 * A frame's painted content cached as one device-pixel bitmap at the board's resting zoom.
 * Board redraws (other frames, selection, labels, pan) blit it instead of calling `paint`.
 * Vector paint is used while the zoom changes, for unbounded or rotated content and over
 * canvas or budget limits. Frames off screen paint nothing; far ones drop their bitmap.
 */
export class FrameBitmap {
  private canvas: HTMLCanvasElement | undefined;
  private box: BitmapBox | undefined;
  private content: FrameContent | undefined;

  constructor(private readonly board: BoardPaint) {}

  draw(context: CanvasRenderingContext2D, content: FrameContent, paint: Paint) {
    const m = context.getTransform();
    const extent = sceneExtent(content.tree);
    if (!extent || m.b || m.c) {
      this.release();
      paint(context);
      return;
    }
    const where = placement(extent, m, context.canvas.width, context.canvas.height);
    if (where === "far") this.release();
    // Every paint lies inside the extent, so an off-screen frame shows nothing either way.
    if (where !== "visible") return;
    const settled = this.board.settled(m.a);
    if (
      !this.canvas ||
      !this.box ||
      this.content?.scene !== content.scene ||
      this.content.resources !== content.resources ||
      this.content.tree !== content.tree ||
      !sameScale(this.box.scale, m.a)
    ) {
      this.release();
      // A frame larger than the board canvas stays vector: paintTree already skips its
      // off-screen subtrees, so a whole-frame raster would cost more than it saves.
      const area = context.canvas.width * context.canvas.height;
      if (!settled || !this.raster(content, bitmapBox(extent, m, area), paint)) {
        paint(context);
        return;
      }
    }
    this.blit(context, m);
  }

  /** Frees the bitmap's pixels now (scene change, far away, unmount). */
  release() {
    if (this.canvas && this.box) this.board.release(this.box.width * this.box.height);
    if (this.canvas) this.canvas.width = this.canvas.height = 0;
    this.canvas = this.box = this.content = undefined;
  }

  private raster(content: FrameContent, box: BitmapBox | undefined, paint: Paint) {
    if (!box || !this.board.reserve(box.width * box.height)) return false;
    const canvas = document.createElement("canvas");
    canvas.width = box.width;
    canvas.height = box.height;
    const target = canvas.getContext("2d");
    if (!target) {
      this.board.release(box.width * box.height);
      return false;
    }
    target.setTransform(box.scale, 0, 0, box.scale, box.phaseX - box.x0, box.phaseY - box.y0);
    paint(target);
    this.canvas = canvas;
    this.box = box;
    this.content = content;
    return true;
  }

  private blit(context: CanvasRenderingContext2D, m: DOMMatrix) {
    if (!this.canvas || !this.box) return;
    const at = blitOrigin(this.box, m);
    context.save();
    context.setTransform(1, 0, 0, 1, 0, 0);
    context.drawImage(this.canvas, at.x, at.y);
    context.restore();
  }
}
