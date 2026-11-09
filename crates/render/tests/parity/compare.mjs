// Source-only correctness harness. Generated Cargo files/builds stay outside the repository.
import { mkdtemp, writeFile, readFile } from "node:fs/promises";
import { dirname, join, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import { spawnSync } from "node:child_process";

const here=dirname(fileURLToPath(import.meta.url));
const repo=resolve(here,"../../../..");
const temp=await mkdtemp(join(dirname(repo),"render-validation-"));
const path=(s)=>JSON.stringify(s);
await writeFile(join(temp,"Cargo.toml"),`[package]
name="render-correctness"
version="0.0.0"
edition="2021"
license="MIT"
[lib]
path=${path(join(here,"fixture.rs"))}
crate-type=["cdylib"]
[[bin]]
name="fixture"
path=${path(join(here,"fixture.rs"))}
[dependencies]
opencanvas-render={path=${path(join(repo,"crates/render"))}}
serde_json="=1.0.154"
sha2="=0.10.9"
png="=0.18.1"
moxcms={version="=0.8.1",default-features=false,features=["lut"]}
`);
function run(command,args,encoding="utf8") {
  const r=spawnSync(command,args,{cwd:temp,encoding,maxBuffer:16*1024*1024});
  if(r.status!==0) throw new Error(`${command} failed (${r.status}): ${r.error??r.stderr}`);
  return r.stdout;
}
run("cargo",["generate-lockfile","--offline"]);
run("cargo",["build","--release","--bin","fixture","--locked","--offline"]);
run("cargo",["build","--release","--lib","--target","wasm32-unknown-unknown","--locked","--offline"]);
const native=run(join(temp,"target/release/fixture"),[],null);
const wasm=await readFile(join(temp,"target/wasm32-unknown-unknown/release/render_correctness.wasm"));
const {instance}=await WebAssembly.instantiate(wasm,{});
const packed=instance.exports.fixture_rgba();
const ptr=Number(packed>>32n),len=Number(packed&0xffffffffn);
const actual=new Uint8Array(instance.exports.memory.buffer,ptr,len);
if(actual.length!==native.length) throw new Error("Native/WASM output length mismatch");
let at=0,max=0,total=0;
for(let fixture=0;fixture<4;fixture++) {
  const expected=new DataView(native.buffer,native.byteOffset+at,16);
  const header=new DataView(actual.buffer,actual.byteOffset+at,16);
  for(let i=0;i<4;i++) if(expected.getUint32(i*4,true)!==header.getUint32(i*4,true)) throw new Error(`Fixture ${fixture} geometry/stats mismatch`);
  const pixels=header.getUint32(0,true)*header.getUint32(4,true);
  at+=16;let local=0;
  for(let i=0;i<pixels*4;i++) local=Math.max(local,Math.abs(native[at+i]-actual[at+i]));
  max=Math.max(max,local);total+=pixels;
  console.log(`fixture ${fixture}: ${pixels} pixels; max RGBA Δ=${local}`);
  at+=pixels*4;
}
if(at!==actual.length || max>1) throw new Error(`Native/WASM correctness gate failed: max Δ=${max}`);
console.log(`PASS: ${total} pixels across 4 fixtures; max Δ=${max}; generated files: ${temp}`);
