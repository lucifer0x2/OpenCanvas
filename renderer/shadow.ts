/** CSS box shadows in scene units; blur/offset are explicitly transformed to device pixels. */
import type { Bounds, Quad, SceneNode, SceneShadow } from "./types";
import { clampedCorners } from "./corners";
import { strokeEdges } from "./stroke";

export function spreadRadius(r: number, s: number) {
  if (!r) return 0;
  return s <= 0 ? Math.max(0,r+s) : r >= s ? r+s : r+s*(1+(r/s-1)**3);
}
const opaque = (fill: SceneNode["fill"]) => typeof fill === "string" &&
  (/^#([\da-f]{3}|[\da-f]{6})$/i.test(fill) || /^#([\da-f]{3}f|[\da-f]{6}ff)$/i.test(fill));
const scale = (c: CanvasRenderingContext2D) => { const m=c.getTransform(); return Math.sqrt(Math.abs(m.a*m.d-m.b*m.c)); };
function path(c: CanvasRenderingContext2D,b: Bounds,r: Quad, dx=0,dy=0) {
  c.roundRect(b.x+dx,b.y+dy,b.width,b.height,r);
}
function shadow(c: CanvasRenderingContext2D,s: SceneShadow,k:number,dx:number) {
  c.shadowColor=s.color; c.shadowBlur=s.blur*k;
  c.shadowOffsetX=dx*k; c.shadowOffsetY=0;
  c.fillStyle="#000";
}
/** Conservative extent, used for clipping scratch work and group opacity allocation. */
export function shadowBounds(n: SceneNode): Bounds | null {
  let left=Infinity,top=Infinity,right=-Infinity,bottom=-Infinity;
  for (const s of n.shadow ?? []) if (!s.inset) {
    const p=s.spread+1.5*s.blur+1, b=n.bounds;
    left=Math.min(left,b.x+s.x-p); top=Math.min(top,b.y+s.y-p);
    right=Math.max(right,b.x+b.width+s.x+p); bottom=Math.max(bottom,b.y+b.height+s.y+p);
  }
  return left<right && top<bottom ? {x:left,y:top,width:right-left,height:bottom-top} : null;
}
export function paintOuterShadows(c: CanvasRenderingContext2D,n: SceneNode) {
  if (!n.shadow?.length) return;
  const b=n.bounds, r=clampedCorners(b,n.radius), k=scale(c);
  if (!(k>0)) return;
  for (const s of [...n.shadow].reverse()) if (!s.inset) {
    const box={x:b.x-s.spread+s.x,y:b.y-s.spread+s.y,width:b.width+2*s.spread,height:b.height+2*s.spread};
    if (box.width<=0 || box.height<=0) continue;
    const radii=r.map(v=>spreadRadius(v,s.spread)) as Quad;
    c.save();
    if (!opaque(n.fill)) {
      const ext=shadowBounds(n)!;
      c.beginPath(); c.rect(ext.x-2,ext.y-2,ext.width+4,ext.height+4); path(c,b,r); c.clip("evenodd");
    }
    c.beginPath();
    if (!s.blur) { path(c,box,radii); c.fillStyle=s.color; c.fill(); }
    else {
      // Move the source beyond the current canvas in device space, preserving its shadow.
      const m=c.getTransform(), d=Math.max(0,box.x+box.width+m.e/k)+1.5*s.blur+c.canvas.width/k+2;
      shadow(c,s,k,d); path(c,box,radii,-d); c.fill();
    }
    c.restore();
  }
}
export function paintInsetShadows(c: CanvasRenderingContext2D,n: SceneNode) {
  if (!n.shadow?.some(s=>s.inset)) return;
  const b=n.bounds, r=clampedCorners(b,n.radius), k=scale(c);
  const edge=n.stroke?.align === "inside" ? strokeEdges(b,r,n.stroke.width,"inside") : null;
  const p=edge ? {x:edge.i.x,y:edge.i.y,width:edge.i.w,height:edge.i.h} : b;
  const radii=edge ? edge.iR : r.map(v=>({x:v,y:v}));
  if (p.width<=0 || p.height<=0 || !(k>0)) return;
  for (const s of [...n.shadow].reverse()) if (s.inset) {
    c.save(); c.beginPath(); c.roundRect(p.x,p.y,p.width,p.height,radii); c.clip();
    const m=Math.abs(s.x)+Math.abs(s.y)+Math.abs(s.spread)+1.5*s.blur+2;
    const d=s.blur ? p.width+2*m+c.canvas.width/k+10 : 0;
    c.beginPath(); c.rect(p.x-m-d,p.y-m,p.width+2*m,p.height+2*m);
    const w=p.width-2*s.spread,h=p.height-2*s.spread;
    if (w>0 && h>0) c.roundRect(p.x+s.x+s.spread-d,p.y+s.y+s.spread,w,h,
      radii.map(v=>({x:Math.max(0,v.x-s.spread),y:Math.max(0,v.y-s.spread)})));
    if (s.blur) shadow(c,s,k,d); else c.fillStyle=s.color;
    c.fill("evenodd"); c.restore();
  }
}
