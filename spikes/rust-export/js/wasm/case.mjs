import { get, stats, compare } from "./metrics.mjs";

export async function runCase(module, font, png, name, scale, iters) {
  const caseName = `${name}-s${scale}`;
  let t = performance.now();
  const [json, native] = await Promise.all([get(`/scenes/${name}.json`), get(`/native/${caseName}.rgba`)]);
  const caseFetchMs = performance.now() - t;
  t = performance.now();
  const { exports: x } = await WebAssembly.instantiate(module, {});
  const instantiateMs = performance.now() - t;
  const check = (operation, code) => { if (code !== 0) throw new Error(`${caseName}: ${operation} returned ${code}`); };
  const put = (b) => {
    const p = x.alloc(b.length);
    if (!p) throw new Error(`${caseName}: alloc failed`);
    new Uint8Array(x.memory.buffer, p, b.length).set(b);
    return [p, b.length];
  };
  t = performance.now();
  const [f, p, s] = [put(font), put(png), put(json)];
  const inputCopyMs = performance.now() - t;
  let decodeMs, initMs, sceneParseMs;
  try {
    t = performance.now();
    check("decode_image", x.decode_image(...p));
    decodeMs = performance.now() - t;
    t = performance.now();
    check("init_font", x.init_font(...f));
    initMs = performance.now() - t;
    t = performance.now();
    check("load_scene", x.load_scene(...s));
    sceneParseMs = performance.now() - t;
  } finally { x.dealloc(...f); x.dealloc(...p); x.dealloc(...s); }
  const render = () => {
    const start = performance.now();
    check("render", x.render(scale));
    return performance.now() - start;
  };
  const cold = render();
  const warm = Array.from({ length: iters }, render);
  t = performance.now();
  // Snapshot bytes before any more WASM calls (memory.grow invalidates JS views).
  const rgba = new Uint8Array(x.memory.buffer, x.pixels_ptr(), x.pixels_len()).slice();
  const outputCopyMs = performance.now() - t;
  const size = [x.width(), x.height()];
  if (rgba.length !== size[0] * size[1] * 4) throw new Error(`${caseName}: incomplete RGBA output`);
  const vsNative = compare(native, rgba, size[0]);
  return { rgba, result: { case: caseName, size, caseFetchMs, instantiateMs, inputCopyMs, decodeMs, initMs,
    sceneParseMs, outputCopyMs, ...stats(cold, warm), vsNative } };
}
