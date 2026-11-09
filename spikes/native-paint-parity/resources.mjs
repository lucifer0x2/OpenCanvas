import { readFile, writeFile, mkdir, copyFile } from "node:fs/promises";
import { resolve, join } from "node:path";
import { hash } from "./scene.mjs";
const fonts = [
  ["noto", "renderer/fonts/NotoSans.ttf", "OpenCanvas Noto Sans", 400, "normal", "bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d"],
  ["lato-regular", "fixtures/fonts/Lato-Regular.ttf", "Lato", 400, "normal", "d636e4683231f931eda222d588e944d082bfd3bdba02f928bee461c0f185b251"],
  ["lato-bold", "fixtures/fonts/Lato-Bold.ttf", "Lato", 700, "normal", "8a0aace75d33794eece4b28187bfc1df0bbd2888b5d8a56e01788c8d65d16be1"],
  ["lato-italic", "fixtures/fonts/Lato-Italic.ttf", "Lato", 400, "italic", "e399c44efe1387100531d26c7e4800c5d12251b890d6654a3098c7c679cb1786"],
  ["inter", "spikes/native-render-parity/out/fonts/inter.ttf", "Inter", 400, "normal", "4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf"],
];
// Original harness-authored JPEG with a minimal TIFF IFD orientation=6 EXIF segment.
function orientation6(jpeg) {
  const payload = Buffer.alloc(32); payload.write("Exif\0\0", "binary"); payload.write("II", 6);
  payload.writeUInt16LE(42, 8); payload.writeUInt32LE(8, 10); payload.writeUInt16LE(1, 14);
  payload.writeUInt16LE(0x112, 16); payload.writeUInt16LE(3, 18); payload.writeUInt32LE(1, 20); payload.writeUInt16LE(6, 24);
  const segment = Buffer.alloc(4); segment[0] = 255; segment[1] = 225; segment.writeUInt16BE(payload.length + 2, 2);
  return Buffer.concat([jpeg.subarray(0, 2), segment, payload, jpeg.subarray(2)]);
}
export async function prepareResources(page, root, out) {
  const dir = join(out, "resources"); await mkdir(dir, { recursive: true });
  const result = { fonts: [], assets: [], sources: [] };
  for (const [id, source, family, weight, style, expected] of fonts) {
    const bytes = await readFile(resolve(root, source)), digest = hash(bytes);
    if (digest !== expected) throw Error(`Pinned font mismatch: ${source}`);
    await writeFile(join(dir, `${id}.ttf`), bytes);
    result.fonts.push({ id, family, weight, style, hash: digest, url: `/resources/${id}.ttf` });
    result.sources.push({ id, source, sha256: digest, bytes: bytes.length, license: "OFL-1.1", origin: id === "noto" ? "renderer/fonts/manifest.json" : id === "inter" ? "https://raw.githubusercontent.com/rsms/inter/e3a3d4c57d5ecc01453a575621882a384c1995a3/docs/font-files/InterVariable.ttf" : "fixtures/fonts/README.md" });
  }
  await copyFile(join(root, "renderer/fonts/OFL.txt"), join(dir, "Noto-OFL.txt"));
  await copyFile(join(root, "fixtures/fonts/Lato-OFL.txt"), join(dir, "Lato-OFL.txt"));
  const interLicense = await readFile(join(root, "spikes/native-render-parity/out/fonts/inter-license.txt"));
  if (hash(interLicense) !== "262481e844521b326f5ecd053e59b98c8b2da78c8ee1bdbb6e8174305e54935a") throw Error("Inter license hash mismatch");
  await writeFile(join(dir, "Inter-OFL.txt"), interLicense);
  const encoded = await page.evaluate(() => {
    const canvas = document.createElement("canvas"); canvas.width = 63; canvas.height = 41;
    const ctx = canvas.getContext("2d"), data = ctx.createImageData(63, 41);
    for (let y = 0; y < 41; y++) for (let x = 0; x < 63; x++) {
      const i = (y * 63 + x) * 4;
      data.data.set([x < 21 ? 245 : 20, y < 20 ? 35 : 210, x > 40 ? 240 : 30, x < 8 && y < 8 ? 0 : 255], i);
    }
    ctx.putImageData(data, 0, 0);
    return ["png", "jpeg", "webp"].map(type => [type, canvas.toDataURL(`image/${type}`, 0.94)]);
  });
  for (const [id, url] of encoded) {
    if (!url.startsWith(`data:image/${id};`)) throw Error(`Browser cannot encode ${id}`);
    const bytes = Buffer.from(url.split(",")[1], "base64");
    await addAsset(id, `image/${id}`, bytes);
    if (id === "jpeg") await addAsset("jpeg-orientation6", "image/jpeg", orientation6(bytes));
  }
  await writeFile(join(dir, "broken.png"), "not an image");
  await writeFile(join(dir, "broken.ttf"), "not a font");
  return result;
  async function addAsset(id, mediaType, bytes) {
    await writeFile(join(dir, `${id}.bin`), bytes);
    result.assets.push({ id, mediaType, hash: hash(bytes), url: `/resources/${id}.bin` });
    result.sources.push({ id, source: "resources.mjs synthetic pixel pattern", sha256: hash(bytes), bytes: bytes.length, license: "MIT (repository)", origin: "Chromium canvas encoder; EXIF constructed locally" });
  }
}
