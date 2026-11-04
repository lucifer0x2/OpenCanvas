// UNTESTED (Task 09): written on a Linux VM without Node/Chromium; never run. Web Worker side of
// js/wasm-bench.mjs. Same protocol and cases as wasm-host (wasm-host/src/main.rs): compile once, then per
// case a fresh instance, init (font + PNG), load_scene, one cold render and `iters` warm renders, each
// timed with performance.now(). Compares the RGBA with the native dump served at /native/<case>.rgba.
const get = async (url) => new Uint8Array(await (await fetch(url)).arrayBuffer());

function stats(cold, warm) {
  warm.sort((a, b) => a - b);
  const rank = (q) => warm[Math.max(1, Math.ceil(q * warm.length)) - 1];
  return { coldMs: cold, p50Ms: rank(0.5), p95Ms: rank(0.95), minMs: warm[0], warm: warm.length };
}

function compare(native, wasm) {
  if (native.length !== wasm.length) return { error: `sizes ${native.length} vs ${wasm.length}` };
  let differ = 0, max = 0;
  for (let i = 0; i < native.length; i += 4) {
    let d = 0;
    for (let c = 0; c < 4; c++) d = Math.max(d, Math.abs(native[i + c] - wasm[i + c]));
    if (d > 0) differ++;
    max = Math.max(max, d);
  }
  return { pixelsDiffer: differ, maxDelta: max };
}

self.onmessage = async ({ data: { wasm, cases, iters } }) => {
  try {
    const [bytes, font, png] = await Promise.all([get(wasm), get("/font.ttf"), get("/asset.png")]);
    let t = performance.now();
    const module = await WebAssembly.compile(bytes);
    const compileMs = performance.now() - t;
    const out = [];
    for (const [name, scale] of cases) {
      const json = await get(`/scenes/${name}.json`);
      t = performance.now();
      const { exports: x } = await WebAssembly.instantiate(module, {});
      const instantiateMs = performance.now() - t;
      const put = (b) => { const p = x.alloc(b.length); new Uint8Array(x.memory.buffer, p, b.length).set(b); return [p, b.length]; };
      t = performance.now();
      const [f, p] = [put(font), put(png)];
      if (x.init(f[0], f[1], p[0], p[1]) !== 0) throw new Error("init");
      x.dealloc(...f); x.dealloc(...p);
      const initMs = performance.now() - t;
      const s = put(json);
      if (x.load_scene(...s) !== 0) throw new Error("load_scene");
      x.dealloc(...s);
      t = performance.now();
      if (x.render(scale) !== 0) throw new Error("render");
      const cold = performance.now() - t;
      const warm = [];
      for (let k = 0; k < iters; k++) {
        t = performance.now();
        x.render(scale);
        warm.push(performance.now() - t);
      }
      const rgba = new Uint8Array(x.memory.buffer, x.pixels_ptr(), x.pixels_len());
      const caseName = `${name}-s${scale}`;
      const native = await fetch(`/native/${caseName}.rgba`);
      const vsNative = native.ok ? compare(new Uint8Array(await native.arrayBuffer()), rgba) : null;
      out.push({ case: caseName, size: [x.width(), x.height()], instantiateMs, initMs, ...stats(cold, warm), vsNative });
    }
    self.postMessage({ compileMs, bytes: bytes.length, cases: out });
  } catch (e) {
    self.postMessage({ error: String(e && e.stack || e) });
  }
};
