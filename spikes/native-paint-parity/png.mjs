// Bounded RGBA8/RGB8 PNG reader and RGBA8 writer for harness artifacts only.
import { deflateSync, inflateSync } from "node:zlib";
const signature = Buffer.from([137, 80, 78, 71, 13, 10, 26, 10]);
function crc(bytes) {
  let c = 0xffffffff;
  for (const b of bytes) {
    c ^= b;
    for (let k = 0; k < 8; k++) c = (c >>> 1) ^ (0xedb88320 & -(c & 1));
  }
  return (c ^ 0xffffffff) >>> 0;
}
function chunk(type, data) {
  const name = Buffer.from(type), result = Buffer.alloc(data.length + 12);
  result.writeUInt32BE(data.length); name.copy(result, 4); data.copy(result, 8);
  result.writeUInt32BE(crc(Buffer.concat([name, data])), data.length + 8);
  return result;
}
export function encodePNG({ width, height, data }) {
  if (!Number.isInteger(width) || !Number.isInteger(height) || width < 1 || height < 1 ||
      width * height > 48_000_000 || data.length !== width * height * 4) throw Error("Invalid RGBA raster");
  const header = Buffer.alloc(13);
  header.writeUInt32BE(width); header.writeUInt32BE(height, 4); header[8] = 8; header[9] = 6;
  const rows = Buffer.alloc(height * (width * 4 + 1));
  for (let y = 0; y < height; y++) Buffer.from(data.subarray(y * width * 4, (y + 1) * width * 4)).copy(rows, y * (width * 4 + 1) + 1);
  return Buffer.concat([signature, chunk("IHDR", header), chunk("IDAT", deflateSync(rows)), chunk("IEND", Buffer.alloc(0))]);
}
export function decodePNG(bytes) {
  if (!bytes.subarray(0, 8).equals(signature)) throw Error("PNG signature");
  let at = 8, width, height, channels, ended = false;
  const parts = [];
  while (at + 12 <= bytes.length) {
    const size = bytes.readUInt32BE(at), type = bytes.toString("ascii", at + 4, at + 8);
    if (at + size + 12 > bytes.length) throw Error("Truncated PNG chunk");
    const data = bytes.subarray(at + 8, at + 8 + size);
    if (crc(bytes.subarray(at + 4, at + 8 + size)) !== bytes.readUInt32BE(at + 8 + size)) throw Error("PNG CRC");
    if (type === "IHDR") {
      width = data.readUInt32BE(0); height = data.readUInt32BE(4);
      if (data.length !== 13 || data[8] !== 8 || ![2, 6].includes(data[9]) || data[10] || data[11] || data[12] ||
          !width || !height || width * height > 16_000_000) throw Error("Unsupported PNG header");
      channels = data[9] === 6 ? 4 : 3;
    } else if (type === "IDAT") parts.push(data);
    else if (type === "IEND") { ended = true; break; }
    at += size + 12;
  }
  if (!ended || !channels) throw Error("Incomplete PNG");
  const stride = width * channels, expected = height * (stride + 1);
  const raw = inflateSync(Buffer.concat(parts), { maxOutputLength: expected });
  if (raw.length !== expected) throw Error("PNG scanline length");
  const pixels = Buffer.alloc(height * stride), data = Buffer.alloc(width * height * 4);
  for (let y = 0; y < height; y++) {
    const filter = raw[y * (stride + 1)];
    if (filter > 4) throw Error("PNG filter");
    for (let x = 0; x < stride; x++) {
      const i = y * stride + x, a = x >= channels ? pixels[i - channels] : 0;
      const b = y ? pixels[i - stride] : 0, c = y && x >= channels ? pixels[i - stride - channels] : 0;
      const p = a + b - c, pa = Math.abs(p - a), pb = Math.abs(p - b), pc = Math.abs(p - c);
      const prediction = [0, a, b, Math.floor((a + b) / 2), pa <= pb && pa <= pc ? a : pb <= pc ? b : c][filter];
      pixels[i] = raw[y * (stride + 1) + x + 1] + prediction;
    }
  }
  for (let i = 0; i < width * height; i++) {
    pixels.copy(data, i * 4, i * channels, i * channels + 3);
    data[i * 4 + 3] = channels === 4 ? pixels[i * 4 + 3] : 255;
  }
  return { width, height, data };
}
