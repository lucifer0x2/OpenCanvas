import type { Bounds, Quad } from "./types";
/** Scene arrays are always four corners; legacy scalar scenes retain their old shape. */
export const corners = (radius: number | Quad = 0): Quad =>
  typeof radius === "number" ? [radius, radius, radius, radius] : radius;
const ratio = (side: number, sum: number) => (sum > 0 ? side / sum : Infinity);
/**
 * CSS overlap reduction (css-backgrounds §5.5): one factor f = min(side / sum of its two
 * adjacent radii) scales every corner, measured on the border box `bounds`.
 */
export function clampedCorners(bounds: Bounds, radius: number | Quad = 0): Quad {
  const r = corners(radius), [a, b, c, d] = r;
  const f = Math.min(1, ratio(bounds.width, a + b), ratio(bounds.width, c + d),
    ratio(bounds.height, a + d), ratio(bounds.height, b + c));
  return r.map(v => v * Math.max(0, f)) as Quad;
}
