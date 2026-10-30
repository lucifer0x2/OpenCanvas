import type { Bounds, SceneNode, SceneStroke } from "./types";
import { clampedCorners, corners as cornerRadii } from "./corners.ts";
import { paintRing } from "./stroke-ring.ts";
import { paintSides } from "./stroke-sides.ts";

type Quad = [number, number, number, number];
interface Radius {
  x: number;
  y: number;
}
interface Rect {
  x: number;
  y: number;
  w: number;
  h: number;
}

const ALIGN: Record<string, number> = { inside: 0, center: 0.5, outside: 1 };

/**
 * Outer and inner edges of a stroke like a browser border: the outer edge grows by
 * width·f per side, the inner edge is the outer edge less the width, and corner radii follow
 * per axis (elliptical when adjacent widths differ). Radii are per corner [tl, tr, br, bl].
 * Radii first reduce like CSS on the node's own box, so the ring follows the clamped fill
 * shape; an inside ring is then exactly a CSS border (inner = outer − adjacent width, ≥ 0).
 */
export function strokeEdges(b: Bounds, authored: Quad, w: Quad, align: string) {
  const f = ALIGN[align] ?? 0;
  const radii = clampedCorners(b, authored);
  const [t, r, bo, l] = w;
  const k: [number, number][] = [
    [l, t],
    [r, t],
    [r, bo],
    [l, bo],
  ];
  const o: Rect = {
    x: b.x - l * f,
    y: b.y - t * f,
    w: b.width + (l + r) * f,
    h: b.height + (t + bo) * f,
  };
  const oR: Radius[] = k.map(([h, v], n) =>
    radii[n] > 0
      ? { x: radii[n] + h * f, y: radii[n] + v * f }
      : { x: 0, y: 0 },
  );
  const i: Rect = { x: o.x + l, y: o.y + t, w: o.w - l - r, h: o.h - t - bo };
  const iR: Radius[] = oR.map((p, n) => ({
    x: Math.max(0, p.x - k[n][0]),
    y: Math.max(0, p.y - k[n][1]),
  }));
  return { o, oR, i, iR, k };
}

export type Edges = ReturnType<typeof strokeEdges>;
/** Paints a node's stroke in scene units: one ring for one color, else side by side. */
export function paintStroke(
  context: CanvasRenderingContext2D,
  node: SceneNode,
) {
  const stroke: SceneStroke | undefined = node.stroke;
  if (!stroke) return;
  const radius = node.radius ?? 0;
  const e = strokeEdges(
    node.bounds,
    cornerRadii(radius),
    stroke.width,
    stroke.align,
  );
  if (e.o.w <= 0 || e.o.h <= 0) return;
  const sides = [0, 1, 2, 3].filter((n) => stroke.width[n] > 0);
  const colors = sides.map((n) => stroke.color[n]);
  if (colors.every((c) => c === colors[0])) {
    paintRing(context, e, colors[0]);
    return;
  }
  paintSides(context, e, stroke);
}

/** The stroke's outer edge in scene units (past bounds when outside or center); null if unpainted. */
export function strokeOuter(node: SceneNode): Bounds | null {
  const stroke = node.stroke;
  if (!stroke || !stroke.width.some((w) => w > 0)) return null;
  const { o } = strokeEdges(
    node.bounds,
    [0, 0, 0, 0],
    stroke.width,
    stroke.align,
  );
  return o.w > 0 && o.h > 0
    ? { x: o.x, y: o.y, width: o.w, height: o.h }
    : null;
}
