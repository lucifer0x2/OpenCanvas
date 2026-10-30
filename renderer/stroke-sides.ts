/**
 * Several stroke colors, painted like Chromium's per-side CSS border painting: each side fills
 * the ring clipped to its wedge, in Chromium's order (more transparent first, then top/bottom
 * before right/left), so seams blend the same way.
 */
import type { SceneStroke } from "./types";
import type { Edges } from "./stroke";
import { paintRing } from "./stroke-ring.ts";

interface Point {
  x: number;
  y: number;
}
type Rect = Edges["o"];

const corners = (r: Rect): Point[] => [
  { x: r.x, y: r.y },
  { x: r.x + r.w, y: r.y },
  { x: r.x + r.w, y: r.y + r.h },
  { x: r.x, y: r.y + r.h },
];
/**
 * Per-side wedges [top, right, bottom, left]. Like Chromium's border side clip, each corner
 * splits along the line from the outer corner through the inner corner, up to the chord of
 * the inner arc, so the seam keeps that angle however large the radii are.
 */
function wedges({ o, i, iR, k }: Edges): Point[][] {
  const O = corners(o);
  const E = corners(i).map((p, n) => {
    const [h, v] = k[n];
    const { x: rx, y: ry } = iR[n];
    // p + (h, v)·t meets the chord x/rx + y/ry = 1 (in the corner's inward axes).
    const t = rx > 0 && ry > 0 && h + v > 0 ? 1 / (h / rx + v / ry) : 0;
    return { x: p.x + (p.x - O[n].x) * t, y: p.y + (p.y - O[n].y) * t };
  });
  return [0, 1, 2, 3].map((n) => {
    const m = (n + 1) % 4;
    return [O[n], O[m], E[m], E[n]];
  });
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
  paintRing(context, e, color, false);
  context.restore();
}

const NORMAL: Point[] = [{ x: 0, y: -1 }, { x: 1, y: 0 }, { x: 0, y: 1 }, { x: -1, y: 0 }];

/**
 * Side n's wedge, with each seam pushed `d` (about a device pixel) into the neighbor where
 * Chromium leaves no blended seam: a same-colored or unpainted neighbor, or, at a square
 * corner, a neighbor painted later (it antialiases over this side). A rounded corner between
 * different colors keeps the exact seam on both sides, as Chromium antialiases both clips.
 */
function sideClip(e: Edges, parts: Point[][], n: number, stroke: SceneStroke, d: number,
  rank: number[]) {
  const [a, b, c, f] = parts[n];
  // Outer points also move 2s outward so the pushed wedge still covers the outer edge.
  const shift = (p: Point, m: number, corner: number, out: number) => {
    const same = stroke.width[m] === 0 || stroke.color[m] === stroke.color[n];
    const square = e.oR[corner].x === 0 || e.oR[corner].y === 0;
    const s = same || (square && rank[m] > rank[n]) ? d : 0;
    const [N, M] = [NORMAL[n], NORMAL[m]];
    return { x: p.x + s * (M.x - N.x + out * N.x), y: p.y + s * (M.y - N.y + out * N.y) };
  };
  const prev = (n + 3) % 4, next = (n + 1) % 4;
  return [shift(a, prev, n, 2), shift(b, next, next, 2), shift(c, next, next, 0), shift(f, prev, n, 0)];
}


const alpha = (color: string) => {
  const h = /^#([\da-f]+)$/i.exec(color)?.[1] ?? "";
  return h.length === 4 ? parseInt(h[3] + h[3], 16) : h.length === 8 ? parseInt(h.slice(6), 16) : 255;
};

/** All side colors: paint order, seam pushes (opaque only, ~1 device pixel), one wedge each. */
export function paintSides(context: CanvasRenderingContext2D, e: Edges, stroke: SceneStroke) {
  const sides = [0, 1, 2, 3].filter((n) => stroke.width[n] > 0);
  const order = [...sides].sort((a, b) =>
    alpha(stroke.color[a]) - alpha(stroke.color[b]) || (a % 2) - (b % 2) || a - b);
  const rank = [4, 4, 4, 4];
  order.forEach((n, i) => (rank[n] = i));
  const parts = wedges(e);
  const m = context.getTransform(), k = Math.sqrt(Math.abs(m.a * m.d - m.b * m.c));
  const d = sides.every((n) => alpha(stroke.color[n]) === 255) && k > 0 ? 1 / k : 0;
  for (const n of order)
    fillWedge(context, e, sideClip(e, parts, n, stroke, d, rank), stroke.color[n]);
}
