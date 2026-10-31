/**
 * Fills one stroke ring (outer edge minus inner edge) the way Chromium paints the equivalent
 * CSS border, so canvas, PNG and the DOM preview agree at the pixel level.
 */
import type { Edges } from "./stroke";

const fits = ({ i, iR }: Edges) =>
  iR[0].x + iR[1].x <= i.w && iR[3].x + iR[2].x <= i.w &&
  iR[0].y + iR[3].y <= i.h && iR[1].y + iR[2].y <= i.h;

/**
 * Uniform width w whose rounded corners all exceed w/2: Chromium paints such a CSS border as a
 * w-wide stroke of the mid-line rounded rect (same geometry, different edge coverage).
 */
function midline({ o, oR, k }: Edges): number {
  const w = k[0][1];
  const uniform = w > 0 && k.every(([h, v]) => h === w && v === w);
  return uniform && o.w > w && o.h > w &&
    oR.every((r) => (r.x === 0 && r.y === 0) || (r.x > w / 2 && r.y > w / 2)) ? w : 0;
}

/**
 * Inner radii past the inner box (oversized asymmetric radii with unequal widths) are not
 * reduced again, as in CSS: each inner corner keeps its own ellipse, cut by the inner box.
 * The clip keeps everything outside the inner box plus each corner's area outside its
 * ellipse; every piece winds like the outer rect, so nonzero unions them.
 */
function clipOutsideInner(c: CanvasRenderingContext2D, { o, i, iR }: Edges) {
  const q = Math.PI / 2, [tl, tr, br, bl] = iR, r = i.x + i.w, b = i.y + i.h;
  c.beginPath();
  c.rect(o.x - 1, o.y - 1, o.w + 2, o.h + 2);
  c.rect(r, i.y, -i.w, i.h);
  const piece = (x: number, y: number, ax: number, ay: number, cx: number, cy: number,
    rx: number, ry: number, from: number, to: number) => {
    if (!(rx > 0 && ry > 0)) return;
    c.moveTo(x, y); c.lineTo(ax, ay); c.ellipse(cx, cy, rx, ry, 0, from, to, true); c.closePath();
  };
  piece(i.x, i.y, i.x + tl.x, i.y, i.x + tl.x, i.y + tl.y, tl.x, tl.y, -q, -Math.PI);
  piece(r, i.y, r, i.y + tr.y, r - tr.x, i.y + tr.y, tr.x, tr.y, 0, -q);
  piece(r, b, r - br.x, b, r - br.x, b - br.y, br.x, br.y, q, 0);
  piece(i.x, b, i.x, b - bl.y, i.x + bl.x, b - bl.y, bl.x, bl.y, Math.PI, q);
  c.clip("nonzero");
}

/** `single`: the whole border is one color (Chromium's fast path may stroke the mid-line). */
export function paintRing(c: CanvasRenderingContext2D, e: Edges, color: string, single = true) {
  const w = single ? midline(e) : 0, h = w / 2;
  c.save();
  c.beginPath();
  if (w) {
    c.roundRect(e.o.x + h, e.o.y + h, e.o.w - w, e.o.h - w,
      e.oR.map((r) => ({ x: Math.max(0, r.x - h), y: Math.max(0, r.y - h) })));
    c.lineWidth = w;
    c.lineJoin = "miter";
    c.strokeStyle = color;
    c.stroke();
  } else if (e.i.w > 0 && e.i.h > 0 && !fits(e)) {
    clipOutsideInner(c, e);
    c.beginPath();
    c.roundRect(e.o.x, e.o.y, e.o.w, e.o.h, e.oR);
    c.fillStyle = color;
    c.fill();
  } else {
    c.roundRect(e.o.x, e.o.y, e.o.w, e.o.h, e.oR);
    if (e.i.w > 0 && e.i.h > 0) c.roundRect(e.i.x, e.i.y, e.i.w, e.i.h, e.iR);
    c.fillStyle = color;
    c.fill("evenodd");
  }
  c.restore();
}
