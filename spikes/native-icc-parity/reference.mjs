// Independent arithmetic reference, not a second CMS dependency or replacement oracle.
const clamp = x => Math.min(1, Math.max(0, x));
const linear = x => x <= 0.04045 ? x / 12.92 : ((x + 0.055) / 1.055) ** 2.4;
const encoded = x => Math.round(255 * (clamp(x) <= 0.0031308 ? 12.92 * clamp(x) : 1.055 * clamp(x) ** (1 / 2.4) - 0.055));
const mul = (m, v) => m.map(row => row.reduce((sum, x, i) => sum + x * v[i], 0));
// Display-P3 linear -> XYZ D65 -> linear sRGB (standard primary/white coordinates).
const p3ToSRGB = [[1.22494018, -0.22494018, 0], [-0.04205695, 1.04205695, 0], [-0.01963755, -0.07863605, 1.09827360]];
// ICC XYZ PCS D50 -> D65 (Bradford), then XYZ D65 -> sRGB.
const d50ToD65 = [[0.9554734, -0.0230985, 0.0632593], [-0.0283697, 1.0099955, 0.0210414], [0.0123140, -0.0205077, 1.3303659]];
const xyzToSRGB = [[3.24096994, -1.53738318, -0.49861076], [-0.96924364, 1.87596750, 0.04155506], [0.05563008, -0.20397696, 1.05697151]];
export function expected(f, x) {
  if (f.profile === "cmyk-constant-xyz") {
    // ICC lut16 XYZ encoding: 32768 represents 1.0. Unit fixture's normalized 70/255.
    return [...mul(xyzToSRGB, mul(d50ToD65, Array(3).fill(70 * 257 / 32768))).map(encoded), 255];
  }
  const i = Math.floor(x / 8), a = f.alpha ? [128, 64, 192, 0][i] : 255;
  if (f.profile === "gray-linear") {
    const g = [32, 96, 128, 192][i], value = f.tagged ? encoded(g / 255) : g;
    return [value, value, value, a];
  }
  let rgb = [[180, 100, 100], [80, 170, 110], [100, 120, 190], [128, 128, 128]][i];
  if (f.tagged && f.profile === "display-p3") rgb = mul(p3ToSRGB, rgb.map(v => linear(v / 255))).map(encoded);
  return [...rgb, a];
}

export function checkReference(f, raster) {
  if (f.profile === "cmyk-constant-xyz" && !f.tagged) return { checked: false, reason: "Untagged CMYK is browser-convention dependent" };
  let maxDelta = 0, alphaMaxDelta = 0, worst = null;
  for (let y = 0; y < raster.height; y++) for (let x = 0; x < raster.width; x++) {
    const want = expected(f, x), got = [...raster.data.subarray((y * raster.width + x) * 4, (y * raster.width + x) * 4 + 4)];
    alphaMaxDelta = Math.max(alphaMaxDelta, Math.abs(got[3] - want[3]));
    for (let c = 0; c < 4; c++) {
      if (c < 3 && !want[3] && !got[3]) continue;
      const delta = Math.abs(got[c] - want[c]);
      if (delta > maxDelta) { maxDelta = delta; worst = { x, y, got, expected: want }; }
    }
  }
  // Independent validity check includes premultiply/unpremultiply quantization (alpha>=64).
  return { checked: true, pass: maxDelta <= 4 && alphaMaxDelta === 0, maxDelta, alphaMaxDelta, worst, tolerance: 4 };
}

export function effect(profile, tagged, untagged) {
  let maxRGBDelta = 0, differentPixels = 0;
  if (tagged.width !== untagged.width || tagged.height !== untagged.height) throw Error("Pair dimensions");
  for (let i = 0; i < tagged.data.length; i += 4) {
    if (!tagged.data[i + 3] && !untagged.data[i + 3]) continue;
    const delta = Math.max(...[0, 1, 2].map(c => Math.abs(tagged.data[i + c] - untagged.data[i + c])));
    maxRGBDelta = Math.max(maxRGBDelta, delta); differentPixels += +(delta > 8);
  }
  return { maxRGBDelta, differentPixels, pass: profile === "srgb" ? maxRGBDelta <= 4 : maxRGBDelta > 8 };
}
