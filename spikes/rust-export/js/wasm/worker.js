// Web Worker only: network/read, compile, instantiate, decode/init and paint remain separate.
import { get } from "./metrics.mjs";
import { runCase } from "./case.mjs";

self.onmessage = async ({ data: { wasm, cases, iters } }) => {
  try {
    const started = performance.now();
    const [bytes, font, png] = await Promise.all([get(wasm), get("/font.ttf"), get("/asset.png")]);
    const inputFetchMs = performance.now() - started;
    const t = performance.now();
    const module = await WebAssembly.compile(bytes);
    const compileMs = performance.now() - t;
    const out = [];
    for (const [name, scale] of cases) {
      const { result, rgba } = await runCase(module, font, png, name, scale, iters);
      out.push(result);
      // Transfer a real RGBA buffer, separately from paint/parity. No canvas/ImageBitmap is involved.
      self.postMessage({ transferProbe: { case: result.case, buffer: rgba.buffer,
        sentAt: performance.timeOrigin + performance.now() } }, [rgba.buffer]);
    }
    self.postMessage({ inputFetchMs, compileMs, bytes: bytes.length, crossOriginIsolated, cases: out });
  } catch (e) {
    self.postMessage({ error: String(e?.stack || e) });
  }
};
