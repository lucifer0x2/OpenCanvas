/** Whether one text node's own paints can cover a pixel twice, so node opacity needs a layer. */
import type { SceneNode } from "./types";

/**
 * CSS composites a translucent element once; canvas `fillText`/`fillRect` with `globalAlpha`
 * blends every glyph and decoration separately. They differ wherever two of the node's paints
 * overlap: a decoration across glyphs, negative tracking, or lines whose ink meets (small line
 * height). Lines are compared by ink rectangles widened by one scene unit for antialiasing, and
 * conservatively by any vertical overlap. A missing ink rectangle counts as overlapping.
 * Glyphs that overlap by design at zero or positive tracking (script fonts, tight kerning) are
 * not detected.
 */
export function textSelfOverlaps(n: SceneNode): boolean {
  if (n.kind !== "text" || n.lines.length === 0) return false;
  if (n.decoration || (n.letterSpacing ?? 0) < 0) return true;
  if (n.lines.length === 1) return false;
  let bottom = -Infinity;
  for (const line of n.lines) {
    const ink = line.inkBounds;
    if (!ink) return true;
    const top = line.y + ink.y;
    if (top < bottom + 1) return true;
    bottom = Math.max(bottom, top + ink.height);
  }
  return false;
}
