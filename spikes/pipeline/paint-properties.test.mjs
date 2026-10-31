import {before,after,test} from "node:test";
import assert from "node:assert/strict";
import {readFile,mkdir} from "node:fs/promises";
import {build} from "esbuild";
import {PipelineWorker} from "../../worker/pipeline.mjs";
import {node} from "./fixtures.mjs";
import {cases,input,cssOracle,budgets} from "./paint-fixtures.mjs";
let worker,preview;
before(async()=>{
  preview=(await build({stdin:{contents:'export {buildSceneDom} from "./web/src/preview/sceneDom.ts"; export {stateStyles} from "./web/src/preview/stateStyles.ts";',resolveDir:process.cwd()},bundle:true,format:"esm",write:false})).outputFiles[0].text;
  worker=await PipelineWorker.start({resourceHandler:async(req,res)=>{
    if(req.url!=="/paint-preview.js")return false;res.writeHead(200,{"content-type":"text/javascript"});res.end(preview);return true;
  }});
});
after(async()=>worker?.close());
/** Pixels of PNG `a` against PNG `b`, or the worker's #scene canvas: count with any channel delta > 2, max delta. */
async function compare(a,b=null) {
  return worker.runtime.page.evaluate(async({a,b})=>{
    const read=async b64=>{
      const image=new Image();image.src="data:image/png;base64,"+b64;await image.decode();
      const canvas=document.createElement("canvas");canvas.width=image.width;canvas.height=image.height;
      const c=canvas.getContext("2d");c.drawImage(image,0,0);return c.getImageData(0,0,canvas.width,canvas.height);
    };
    const expected=(await read(a)).data;
    const scene=document.querySelector("#scene");
    const actual=b?(await read(b)).data:scene.getContext("2d").getImageData(0,0,scene.width,scene.height).data;
    let max=0,pixels=0;
    for(let i=0;i<actual.length;i+=4){let d=0;for(let k=0;k<4;k++)d=Math.max(d,Math.abs(actual[i+k]-expected[i+k]));max=Math.max(d,max);if(d>2)pixels++;}
    return {max,pixels};
  },{a:a.toString("base64"),b:b?.toString("base64")});
}
const diff=async page=>compare(await page.screenshot({type:"png"}));
function check(result,label,scale,kind="edge") {
  const b=budgets[kind];
  console.log(`${label}@${scale} [${kind}]: pixels>2=${result.pixels} max=${result.max}`);
  assert.ok(result.pixels<=b.pixels*scale&&result.max<=b.max,`${label}@${scale} ${JSON.stringify(result)} exceeds ${kind}`);
}
async function cssPage(scale,html) {
  const page=await worker.runtime.browser.newPage({viewport:{width:480,height:300},deviceScaleFactor:scale});
  await page.setContent(`<!doctype html><style>body{margin:0;background:white}</style>${html}`);return page;
}
async function showPreview(page,scene,css) {
  await page.goto(worker.runtime.baseURL);
  await page.evaluate(async({scene,css})=>{
    const {buildSceneDom}=await import("/paint-preview.js");
    document.head.insertAdjacentHTML("beforeend",`<style>body{margin:0;background:#fff}${css}</style>`);
    document.body.replaceChildren(buildSceneDom(scene,{fonts:new Map(),images:new Map()},new Set(scene.nodes.filter(n=>n.stroke).map(n=>n.id))).root);
  },{scene,css});
}
test("accepted paint compares with real CSS and actual preview at device scales 1 and 2",async()=>{
  const css=await readFile("web/src/preview/preview.css","utf8");
  await mkdir(".runtime/paint-properties",{recursive:true});
  for(const [label,n,kind] of cases)for(const scale of [1,2]){
    const scene=(await worker.compute(input([n]))).scene;await worker.render(scene,{scale});
    const page=await cssPage(scale,cssOracle([n]));
    try{
      const reference=await page.screenshot({type:"png"});
      check(await compare(reference),label,scale,kind);
      if(label==="S10"&&scale===1)await page.screenshot({path:".runtime/paint-properties/css.png"});
      await showPreview(page,scene,css);
      // The preview is CSS itself, so it must match the CSS reference within the edge budget.
      check(await compare(reference,await page.screenshot({type:"png"})),`${label}-preview`,scale);
      if(label==="S10"&&scale===1){await page.screenshot({path:".runtime/paint-properties/preview.png"});await worker.runtime.page.locator("#scene").screenshot({path:".runtime/paint-properties/canvas.png"});}
    }finally{await page.close();}
  }
});
const sample=(points)=>worker.runtime.page.evaluate(points=>{
  const c=document.querySelector("#scene").getContext("2d");return points.map(([x,y])=>[...c.getImageData(x,y,1,1).data]);
},points);
test("an opacity layer never clips its subtree's shadows; only an authored clip does",async()=>{
  const shadow=node("child","group","rectangle",{x:0,y:0,width:160,height:80,fill:"#fff",radius:12,shadow:[{y:4,blur:12,color:"#0004"}]});
  for(const clip of [false,true])for(const scale of [1,2]){
    const group=node("group","frame","group",{x:40,y:40,width:160,height:82,opacity:.5,clip});
    const nodes=[group,shadow,node("over","group","rectangle",{x:10,y:10,width:20,height:20,fill:"#00f"})];
    const scene=(await worker.compute(input(nodes))).scene;const result=await worker.render(scene,{scale});
    const page=await cssPage(scale,cssOracle(nodes));try{check(await diff(page),`group-clip${clip}`,scale);}finally{await page.close();}
    assert.ok(result.draw.opacityLayers>=1);
    // 6px below the group's bounds: inside the shadow's blur, outside the group (layer) box.
    const [below]=await sample([[120*scale,(40+82+6)*scale]]);
    if(clip)assert.deepEqual(below,[255,255,255,255],"authored clip cuts the shadow");
    else assert.ok(below[0]<255,`opacity layer keeps the shadow outside its box: ${below}`);
  }
});
test("export viewport stays layout bounds: an outer shadow at the target edge is cropped, not clipped",async()=>{
  // Flush with the frame's bottom-right corner; the shadow extends past the viewport.
  const edge=node("edge","frame","rectangle",{x:380,y:220,width:100,height:80,fill:"#fff",radius:8,shadow:[{x:6,y:8,blur:16,color:"#00000080"}]});
  for(const scale of [1,2]){
    const scene=(await worker.compute(input([edge]))).scene;await worker.render(scene,{scale});
    assert.deepEqual(await worker.runtime.page.evaluate(()=>{const c=document.querySelector("#scene");return [c.width,c.height]}),[480*scale,300*scale]);
    // Same pixels as CSS seen through a 480x300 window: the crop is the export viewport only.
    const page=await cssPage(scale,cssOracle([edge]));try{check(await diff(page),"edge-crop",scale);}finally{await page.close();}
    const [left]=await sample([[372*scale,260*scale]]);
    assert.ok(left[0]<255,`shadow inside the viewport is painted: ${left}`);
  }
});
// Center widths are even: an odd center width puts the preview's overlay box on a half CSS
// pixel, which Chromium snaps (a preview-wide limit of fractional DOM geometry, not paint).
test("outside and center rings follow the clamped fill shape in canvas and preview alike",async()=>{
  const css=await readFile("web/src/preview/preview.css","utf8");
  for(const [align,width,kind] of [["outside",3,"arc"],["outside",[3,9,5,13],"arc"],["center",[2,8,4,12],"arc"],["center",4,"arc"]])for(const scale of [1,2]){
    const n=node("box","frame","rectangle",{x:60,y:60,width:160,height:80,fill:"#fff",radius:[100,20,80,5],stroke:{color:"#000",width,align}});
    const scene=(await worker.compute(input([n]))).scene;await worker.render(scene,{scale});
    const page=await cssPage(scale,"");
    try{await showPreview(page,scene,css);check(await diff(page),`${align}-${width}-preview-vs-canvas`,scale,kind);}finally{await page.close();}
  }
});
