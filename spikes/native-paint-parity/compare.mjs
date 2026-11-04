// No masks: each metric's denominator is every output pixel, including transparency.
export const gate = Object.freeze({ channelDeltaExclusive: 8, pixelFractionExclusive: 0.005 });
function metric() { return { pixelsOver8: 0, maxDelta: 0, maxAt: null, channelOver8: [0, 0, 0, 0], sumDelta: 0 }; }
function add(m, deltas, x, y) {
  const delta = Math.max(...deltas);
  if (delta > gate.channelDeltaExclusive) m.pixelsOver8++;
  if (delta > m.maxDelta) { m.maxDelta = delta; m.maxAt = { x, y, deltas }; }
  deltas.forEach((d, c) => { m.channelOver8[c] += +(d > 8); m.sumDelta += d; });
}
export function compare(a, b, options = {}) {
  if (Object.keys(options).some(k => k !== "category")) throw Error("Masks/threshold overrides are not supported");
  if (a.width !== b.width || a.height !== b.height || a.data.length !== b.data.length || a.data.length !== a.width * a.height * 4)
    throw Error("Raster dimensions differ");
  const totalPixels = a.width * a.height;
  if (!totalPixels) throw Error("Empty raster");
  const rawStraight = metric(), visibleStraight = metric(), compositedWhite = metric();
  const diff = Buffer.alloc(a.data.length), strip = Buffer.alloc(a.data.length * 3);
  for (let p = 0; p < totalPixels; p++) {
    const i = p * 4, x = p % a.width, y = Math.floor(p / a.width);
    const raw = [0, 1, 2, 3].map(c => Math.abs(a.data[i + c] - b.data[i + c]));
    // RGB is undefined only when BOTH alpha values are zero. Alpha always participates.
    const visible = a.data[i + 3] === 0 && b.data[i + 3] === 0 ? [0, 0, 0, raw[3]] : raw;
    const white = [0, 1, 2].map(c => Math.abs(
      Math.round(a.data[i + c] * a.data[i + 3] / 255 + 255 - a.data[i + 3]) -
      Math.round(b.data[i + c] * b.data[i + 3] / 255 + 255 - b.data[i + 3]))).concat(raw[3]);
    add(rawStraight, raw, x, y); add(visibleStraight, visible, x, y); add(compositedWhite, white, x, y);
    for (let c = 0; c < 3; c++) diff[i + c] = Math.min(255, Math.max(visible[c], visible[3]) * 8);
    diff[i + 3] = 255;
    const offset = (y * a.width * 3 + x) * 4;
    a.data.copy(strip, offset, i, i + 4); b.data.copy(strip, offset + a.width * 4, i, i + 4);
    diff.copy(strip, offset + a.width * 8, i, i + 4);
  }
  for (const m of [rawStraight, visibleStraight, compositedWhite]) {
    m.fractionOver8 = m.pixelsOver8 / totalPixels; m.meanChannelDelta = m.sumDelta / (totalPixels * 4);
    delete m.sumDelta;
  }
  const passed = visibleStraight.fractionOver8 < gate.pixelFractionExclusive;
  return {
    report: { totalPixels, gate, rawStraight, visibleStraight, compositedWhite, masks: [],
      thresholdExceeded: !passed, gateApplies: options.category !== "text", verdict: options.category === "text" ? "REVIEW_TEXT" : passed ? "PASS" : "FAIL" },
    diff: { width: a.width, height: a.height, data: diff },
    sideBySide: { width: a.width * 3, height: a.height, data: strip },
  };
}
