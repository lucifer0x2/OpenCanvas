//! CSS gradient geometry and premultiplied sRGB interpolation.
import type { Bounds, Paint } from "./types";
type Stop = [number, string];
function rgba(hex: string): number[] {
  let h = hex.slice(1); if (h.length <= 4) h = [...h].map(c => c+c).join("");
  return [0,2,4].map(i => parseInt(h.slice(i,i+2),16)).concat(h.length === 8 ? parseInt(h.slice(6),16)/255 : 1);
}
const css = (v: number[]) => `rgba(${v[0]},${v[1]},${v[2]},${v[3]})`;
/** Extra stops correct canvas's straight-alpha interpolation without changing hard-stop order. */
export function premultipliedStops(stops: Stop[]): Stop[] {
  const out: Stop[] = [];
  for (let i=0; i<stops.length; i++) {
    const [p,color] = stops[i], a = rgba(color);
    if (a[3] === 0) {
      const prev = i ? rgba(stops[i-1][1]) : a, next = i+1<stops.length ? rgba(stops[i+1][1]) : a;
      out.push([p,css([...prev.slice(0,3),0])], [p,css([...next.slice(0,3),0])]);
    } else out.push([p,color]);
    if (i+1 === stops.length) continue;
    const [q,next] = stops[i+1], b = rgba(next);
    if (a[3] === 0 || b[3] === 0 || a[3] === b[3] || p === q) continue;
    // Adaptive subdivision bounds the curvature even for very unequal nonzero alpha.
    for (let j=1; j<64; j++) {
      const t=j/64, alpha=a[3]*(1-t)+b[3]*t;
      out.push([p+(q-p)*t, css([0,1,2].map(k => (a[k]*a[3]*(1-t)+b[k]*b[3]*t)/alpha).concat(alpha))]);
    }
  }
  return out;
}
export function gradientGeometry(paint: Exclude<Paint,string>, b: Bounds) {
  if (paint.type === "linear") {
    const a=(paint.angle ?? 180)*Math.PI/180, dx=Math.sin(a), dy=-Math.cos(a);
    const len=Math.abs(b.width*dx)+Math.abs(b.height*dy), x=b.x+b.width/2, y=b.y+b.height/2;
    return { x0:x-dx*len/2, y0:y-dy*len/2, x1:x+dx*len/2, y1:y+dy*len/2 };
  }
  const [cx,cy]=paint.center ?? [0.5,0.5], r=paint.radius;
  const [rx,ry]=typeof r === "number" ? [r,r] : r ?? [Math.SQRT2*Math.max(Math.abs(cx*b.width),Math.abs(b.width-cx*b.width)), Math.SQRT2*Math.max(Math.abs(cy*b.height),Math.abs(b.height-cy*b.height))];
  return { x:b.x+cx*b.width, y:b.y+cy*b.height, rx,ry };
}
/** Fills the current path. Radial scaling affects the gradient, never its already-built path. */
export function fillPath(c: CanvasRenderingContext2D, paint: Paint, b: Bounds) {
  if (typeof paint === "string") { c.fillStyle=paint; c.fill(); return; }
  c.save();
  const g=gradientGeometry(paint,b);
  let gradient: CanvasGradient;
  if ("x0" in g && g.x0 !== undefined) gradient=c.createLinearGradient(g.x0,g.y0!,g.x1!,g.y1!);
  else { c.translate(g.x,g.y); c.scale(g.rx/g.ry,1); gradient=c.createRadialGradient(0,0,0,0,0,g.ry); }
  for (const [at,color] of premultipliedStops(paint.stops)) gradient.addColorStop(at,color);
  c.fillStyle=gradient; c.fill(); c.restore();
}
