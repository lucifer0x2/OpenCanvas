// Pure metrics/strict input checks, shared by Worker and host validation/tests.
export async function get(url) {
  const response = await fetch(url);
  if (!response.ok) throw new Error(`${url}: HTTP ${response.status}`);
  const bytes = new Uint8Array(await response.arrayBuffer());
  if (!bytes.length) throw new Error(`${url}: empty input`);
  return bytes;
}

export function stats(cold, samples) {
  if (!samples.length || [cold, ...samples].some((v) => !Number.isFinite(v) || v < 0)) throw new Error("Invalid timing samples");
  const warm = [...samples].sort((a, b) => a - b);
  const rank = (q) => warm[Math.max(1, Math.ceil(q * warm.length)) - 1];
  return { coldMs: cold, p50Ms: rank(0.5), p95Ms: rank(0.95), minMs: warm[0], warm: warm.length, samplesMs: samples };
}

export function compare(native, wasm, width) {
  if (!native.length || native.length !== wasm.length || native.length % 4) throw new Error(`RGBA sizes ${native.length} vs ${wasm.length}`);
  let differ = 0, max = 0;
  const histogram = {}, first = [], channels = [0, 0, 0, 0];
  for (let i = 0; i < native.length; i += 4) {
    let d = 0;
    for (let c = 0; c < 4; c++) {
      const delta = Math.abs(native[i + c] - wasm[i + c]);
      d = Math.max(d, delta);
      channels[c] = Math.max(channels[c], delta);
    }
    if (d > 0) {
      differ++;
      histogram[d] = (histogram[d] ?? 0) + 1;
      if (width && first.length < 8) first.push({ x: (i / 4) % width, y: Math.floor(i / 4 / width),
        native: Array.from(native.slice(i, i + 4)), wasm: Array.from(wasm.slice(i, i + 4)) });
    }
    max = Math.max(max, d);
  }
  return { pixelsDiffer: differ, maxDelta: max, ...(width ? { channels, histogram, first } : {}) };
}

export function validate(result, cases, parity = false) {
  if (result?.error) throw new Error(result.error);
  if (result?.cases?.length !== cases.length) throw new Error("Missing/extra benchmark cases");
  for (const [name, scale] of cases) {
    const found = result.cases.filter((c) => c.case === `${name}-s${scale}`);
    if (found.length !== 1) throw new Error(`Missing/duplicate case ${name}-s${scale}`);
    const c = found[0];
    const [w, h] = { spike: [400, 300], frame: [1440, 900], tile: [720, 450] }[name];
    if (c.size?.[0] !== w * scale || c.size?.[1] !== h * scale) throw new Error(`Wrong dimensions: ${c.case}`);
    if (c.warm !== 20 || [c.coldMs, c.p50Ms, c.p95Ms].some((v) => !Number.isFinite(v) || v < 0)) throw new Error(`Invalid timings: ${c.case}`);
    if (parity && (!Number.isInteger(c.vsNative?.pixelsDiffer) || !Number.isInteger(c.vsNative?.maxDelta))) throw new Error(`Missing parity: ${c.case}`);
  }
}
