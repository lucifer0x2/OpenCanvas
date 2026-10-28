import type { Bounds, SceneNode, SceneStroke } from "./types";

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
interface Point {
  x: number;
  y: number;
}

const ALIGN: Record<string, number> = { inside: 0, center: 0.5, outside: 1 };

/**
 * Outer and inner edges of a stroke like a browser border: the outer edge grows by
 * width·f per side, the inner edge is the outer edge less the width, and corner radii follow
 * per axis (elliptical when adjacent widths differ). Radii are per corner [tl, tr, br, bl].
 */
export function strokeEdges(b: Bounds, radii: Quad, w: Quad, align: string) {
  const f = ALIGN[align] ?? 0;
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

type Edges = ReturnType<typeof strokeEdges>;
const corners = (r: Rect): Point[] => [
  { x: r.x, y: r.y },
  { x: r.x + r.w, y: r.y },
  { x: r.x + r.w, y: r.y + r.h },
  { x: r.x, y: r.y + r.h },
];
const clamp = (v: number, a: number, b: number) =>
  Math.min(Math.max(v, Math.min(a, b)), Math.max(a, b));

/**
 * Per-side wedges [top, right, bottom, left]. Each corner splits along the line from the outer
 * corner through the inner corner, extended across the inner radius box so arcs get a side.
 */
function wedges({ o, i, iR, k }: Edges): Point[][] {
  const O = corners(o);
  const E = corners(i).map((p, n) => {
    const [h, v] = k[n];
    const s = Math.max(h > 0 ? iR[n].x / h : 0, v > 0 ? iR[n].y / v : 0);
    return {
      x: clamp(p.x + (p.x - O[n].x) * s, i.x, i.x + i.w),
      y: clamp(p.y + (p.y - O[n].y) * s, i.y, i.y + i.h),
    };
  });
  return [0, 1, 2, 3].map((n) => {
    const m = (n + 1) % 4;
    return [O[n], O[m], E[m], E[n]];
  });
}

function ringPath(context: CanvasRenderingContext2D, e: Edges) {
  context.beginPath();
  context.roundRect(e.o.x, e.o.y, e.o.w, e.o.h, e.oR);
  if (e.i.w > 0 && e.i.h > 0)
    context.roundRect(e.i.x, e.i.y, e.i.w, e.i.h, e.iR);
}

function fillWedge(
  context: CanvasRenderingContext2D,
  e: Edges,
  wedge: Point[],
  color: string,
) {
  context.save();
  context.beginPath();
  wedge.forEach((p, n) =>
    n ? context.lineTo(p.x, p.y) : context.moveTo(p.x, p.y),
  );
  context.closePath();
  context.clip();
  ringPath(context, e);
  context.fillStyle = color;
  context.fill("evenodd");
  context.restore();
}

/** True for hex colors without transparency; anything else is treated as translucent. */
const opaque = (color: string) =>
  /^#([0-9a-f]{3}|[0-9a-f]{6})$/i.test(color) ||
  /^#([0-9a-f]{3}f|[0-9a-f]{6}ff)$/i.test(color);

/**
 * Paints a node's stroke in scene units. One color fills the ring once; opaque side colors
 * paint over a base ring (no seams); translucent colors fill each side's wedge alone so alpha
 * is exact.
 */
export function paintStroke(
  context: CanvasRenderingContext2D,
  node: SceneNode,
) {
  const stroke: SceneStroke | undefined = node.stroke;
  if (!stroke) return;
  const radius = node.radius ?? 0;
  const e = strokeEdges(
    node.bounds,
    [radius, radius, radius, radius],
    stroke.width,
    stroke.align,
  );
  if (e.o.w <= 0 || e.o.h <= 0) return;
  const sides = [0, 1, 2, 3].filter((n) => stroke.width[n] > 0);
  const colors = sides.map((n) => stroke.color[n]);
  if (colors.every((c) => c === colors[0])) {
    ringPath(context, e);
    context.fillStyle = colors[0];
    context.fill("evenodd");
    return;
  }
  const parts = wedges(e);
  if (colors.every(opaque)) {
    ringPath(context, e);
    context.fillStyle = colors[0];
    context.fill("evenodd");
    for (const n of sides)
      if (stroke.color[n] !== colors[0])
        fillWedge(context, e, parts[n], stroke.color[n]);
    return;
  }
  for (const n of sides) fillWedge(context, e, parts[n], stroke.color[n]);
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
