import { node } from "./fixtures.mjs";
const linear=(angle,stops=[[0,"#6366f1"],[1,"#8b5cf6"]])=>({type:"linear",angle,stops});
const box=(props={})=>node("box","frame","rectangle",{x:40,y:40,width:160,height:80,fill:"#fff",radius:12,...props});
const shadow=(v,props={})=>box({...props,shadow:v});
export const cases=[
  ["G1",box({height:48,radius:8,fill:linear(180)})],
  ["G2",box({width:240,height:100,fill:linear(135)})],
  ["G3",box({fill:linear(90,[[0,"#ef4444"],[1,"#0000"]])})],
  ["G4",box({fill:linear(90,[[0,"#ff000080"],[1,"#0000ffcc"]])})],
  ["G5",box({width:300,height:150,fill:{type:"radial",stops:[[0,"#f005"],[1,"#00f0"]]}})],
  ["G6",box({fill:{type:"radial",center:[0.5,0],stops:[[0,"#6366f159"],[0.7,"#0000"],[1,"#0000"]]}})],
  ["G7",box({fill:{type:"radial",center:[0.3,0.4],radius:120,stops:[[0,"#f00"],[0.5,"#0f08"],[1,"#00f0"]]}})],
  // 60deg, not 45deg: at 45deg on integer device bounds pixel centres sit exactly on the hard
  // stop, and Chromium's CSS and canvas paths break that tie differently (69 px at scale 2).
  ["G8",box({width:120,height:220,fill:linear(60,[[0,"#f00"],[0.5,"#0f0"],[0.5,"#00f"],[1,"#fff"]])})],
  ["G9",box({fill:{type:"radial",center:[-0.5,1.5],radius:[250,100],stops:[[0,"#f00"],[1,"#00f"]]}})],
  ["S1",shadow([{y:4,blur:12,color:"#00000040"}])],
  ["S2",shadow([{spread:1,color:"#6366f1"}],{height:40,radius:8})],
  ["S3",shadow([{inset:true,y:1,color:"#ffffff66"}],{height:40,radius:6,fill:"#1f2937"})],
  ["S4",shadow([{inset:true,spread:1,color:"#ffffff1a"}],{height:40,radius:6,fill:"#1f2937"})],
  ["S5",shadow([{y:1,blur:2,color:"#0000000d"},{y:10,blur:30,spread:-5,color:"#00000033"}],{width:200,height:120})],
  ["S6",shadow([{y:24,blur:48,color:"#00000059"}],{width:300,height:160,radius:16})],
  ["S7",shadow([{inset:true,y:2,blur:8,color:"#00000040"}],{width:200,height:100,fill:"#f8fafc"})],
  ["S8",shadow([{y:4,blur:12,color:"#000"}],{fill:"#ffffff80"})],
  ["S9",shadow([{spread:8,color:"#94a3b8"}],{width:120,height:60,radius:4,fill:"#e2e8f0"})],
  ["S10",shadow([{inset:true,y:1,blur:3,color:"#ffffff66"},{y:4,blur:12,color:"#0004"}],{fill:linear(135),radius:[24,4,12,0]})],
  ["inset-border",shadow([{inset:true,y:1,color:"#f00"}],{height:40,stroke:{color:"#000",width:1}})],
  // Review point B: oversized asymmetric radii (f = 80/105) with unequal widths; the left
  // inner radii overflow the inner box (73.2 > 72) and must not be reduced a second time.
  ["B-oversized",box({radius:[100,20,80,5],stroke:{color:"#000",width:[3,9,5,13]}}),"arc"],
  ["B-overflow",box({radius:[100,20,80,5],stroke:{color:"#000",width:[1,1,20,1]}}),"arc"],
  ["B-uniform",box({radius:[100,20,80,5],stroke:{color:"#000",width:3}})],
  ["B-sides",box({radius:[100,20,80,5],stroke:{color:["#f00","#00f"],width:[3,9,5,13]}}),"arc"],
  ["stroke-1px",box({stroke:{color:"#000",width:1}})],
  ["stroke-unequal",box({stroke:{color:"#000",width:[1,2,3,4]}})],
  ["stroke-sides",box({stroke:{color:["#f00","#00f"],width:4}}),"seam"],
  ["stroke-keycap",box({width:40,height:20,radius:5,stroke:{color:["#d0d5dd","#d0d5dd","#667085","#d0d5dd"],width:[1,1,2,1]}}),"seam"],
  ["stroke-translucent",box({stroke:{color:["#ff000080","#00ff00","#0000ff","#000"],width:[1,2,3,4]}}),"seam"],
  ["stroke-square",box({radius:0,stroke:{color:["#f00","#0a0","#00f","#000"],width:[2,4,6,8]}}),"seam"],
];
/**
 * Per-pixel budgets (channel delta > 2) for antialiasing that Chromium computes differently in
 * CSS and canvas, never for geometry: a geometric error of a pixel or more shows as deltas near
 * 255. Pixel counts are per CSS pixel area (scaled by scale²); max is the largest channel delta.
 *  - edge: rounded-corner AA. A plain solid 120x220 r12 box already differs from CSS by
 *    19 px (max 42) at scale 2 near its bottom corners, so this is the shape baseline.
 *  - seam: per-side color seams and their corner pixels (polygon clip AA vs Chromium's).
 *  - arc: large asymmetric arcs where canvas and Chromium pick different AA paths (inner
 *    radii overflowing the inner box use a canvas clip; Chromium may fill or stroke a ring).
 */
export const budgets={edge:{pixels:12,max:48},seam:{pixels:80,max:110},arc:{pixels:190,max:96}};
export function input(nodes,width=480,height=300) {
  return {schemaVersion:1,projectId:"paint",revision:1,frameId:"frame",viewport:{width,height},fonts:[],assets:[],nodes:[node("frame",null,"frame",{fill:"#fff",layout:{mode:"free"}}),...nodes]};
}
const px=v=>`${v}px`, quad=v=>Array.isArray(v)?v:[v??0];
function fill(p) {
  if (typeof p==="string") return p;
  if (!p) return "transparent";
  const stops=p.stops.map(([t,c])=>`${c} ${t*100}%`).join(",");
  if(p.type==="linear")return `linear-gradient(${p.angle??180}deg,${stops})`;
  const r=p.radius, shape=typeof r==="number"?`circle ${r}px`:r?`${r[0]}px ${r[1]}px`:"ellipse farthest-corner";
  return `radial-gradient(${shape} at ${(p.center?.[0]??.5)*100}% ${(p.center?.[1]??.5)*100}%,${stops})`;
}
export function cssOracle(nodes) {
  const render=n=>{
    const p=n.props, border=p.stroke;
    const shadows=(p.shadow??[]).map(s=>`${s.inset?"inset ":""}${s.x??0}px ${s.y??0}px ${s.blur??0}px ${s.spread??0}px ${s.color}`).join(",");
    return `<div style="position:absolute;box-sizing:border-box;left:${p.x??0}px;top:${p.y??0}px;width:${p.width??0}px;height:${p.height??0}px;background:${fill(p.fill)};border-radius:${quad(p.radius).map(px).join(" ")};box-shadow:${shadows||"none"};opacity:${p.opacity??1};overflow:${p.clip?"hidden":"visible"};${border?`border-style:solid;border-width:${quad(border.width??1).map(px).join(" ")};border-color:${quad(border.color).join(" ")};`:""}">${nodes.filter(c=>c.parentId===n.id).map(render).join("")}</div>`;
  };
  return nodes.filter(n=>n.parentId==="frame").map(render).join("");
}
