import { readFile, writeFile, mkdir } from "node:fs/promises";
import { ROOT, OUT, json, sha256 } from "./environment.mjs";

const INTER_ROOT = "https://raw.githubusercontent.com/rsms/inter/e3a3d4c57d5ecc01453a575621882a384c1995a3/";
const LATO_ROOT = "https://raw.githubusercontent.com/google/fonts/f3b885d5590e307e02542f1a724cec55d567fdaa/ofl/lato/";
const INTER = {
  font: { url: `${INTER_ROOT}docs/font-files/InterVariable.ttf`, hash: "4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf" },
  license: { url: `${INTER_ROOT}LICENSE.txt`, hash: "262481e844521b326f5ecd053e59b98c8b2da78c8ee1bdbb6e8174305e54935a" },
};

export function tableTags(bytes) {
  if (bytes.length < 12) throw new Error("Truncated SFNT");
  const count = bytes.readUInt16BE(4);
  if (bytes.length < 12 + count * 16) throw new Error("Truncated table directory");
  return Array.from({ length: count }, (_, i) => bytes.toString("ascii", 12 + i * 16, 16 + i * 16)).sort();
}

export async function boundedDownload(url, maxBytes = 2_000_000) {
  const response = await fetch(url, { signal: AbortSignal.timeout(30_000) });
  if (!response.ok || !response.body) throw new Error(`Download failed: ${response.status} ${url}`);
  const chunks = [];
  let length = 0;
  try {
    for await (const chunk of response.body) {
      length += chunk.length;
      if (length > maxBytes) throw new Error(`Download exceeds ${maxBytes} bytes: ${url}`);
      chunks.push(chunk);
    }
  } catch (error) {
    await response.body.cancel().catch(() => {});
    throw error;
  }
  return Buffer.concat(chunks);
}

async function pinnedDownload(name, pin) {
  const path = new URL(`fonts/${name}`, OUT);
  let bytes;
  try { bytes = await readFile(path); } catch (error) { if (error.code !== "ENOENT") throw error; }
  bytes ??= await boundedDownload(pin.url);
  if (sha256(bytes) !== pin.hash) throw new Error(`Pinned SHA mismatch: ${name}`);
  await writeFile(path, bytes);
  return bytes;
}

export async function prepareFonts({ sfTrak = false } = {}) {
  await mkdir(new URL("fonts/", OUT), { recursive: true });
  const noto = await json(`${ROOT}renderer/fonts/manifest.json`);
  const fixtures = [{ id: "noto", family: noto.family, weight: 400, style: "normal",
    bytes: await readFile(`${ROOT}renderer/fonts/NotoSans.ttf`), expected: noto.sha256, sourceURL: noto.source,
    licenseBytes: await readFile(`${ROOT}renderer/fonts/OFL.txt`),
    licenseURL: "https://github.com/google/fonts/blob/main/ofl/notosans/OFL.txt", license: noto.license }];
  for (const [name, weight, style, hash] of [
    ["Regular", 400, "normal", "d636e4683231f931eda222d588e944d082bfd3bdba02f928bee461c0f185b251"],
    ["Bold", 700, "normal", "8a0aace75d33794eece4b28187bfc1df0bbd2888b5d8a56e01788c8d65d16be1"],
    ["Italic", 400, "italic", "e399c44efe1387100531d26c7e4800c5d12251b890d6654a3098c7c679cb1786"],
  ]) fixtures.push({ id: `lato-${name.toLowerCase()}`, family: "Lato", weight, style,
    bytes: await readFile(`${ROOT}fixtures/fonts/Lato-${name}.ttf`), expected: hash,
    sourceURL: `${LATO_ROOT}Lato-${name}.ttf`, licenseBytes: await readFile(`${ROOT}fixtures/fonts/Lato-OFL.txt`),
    licenseURL: `${LATO_ROOT}OFL.txt`, license: "SIL Open Font License 1.1" });
  fixtures.push({ id: "inter", family: "Inter", weight: 400, style: "normal",
    bytes: await pinnedDownload("inter.ttf", INTER.font), expected: INTER.font.hash,
    sourceURL: INTER.font.url, licenseBytes: await pinnedDownload("inter-license.txt", INTER.license),
    licenseURL: INTER.license.url, license: "SIL Open Font License 1.1" });
  if (sfTrak) {
    if (process.platform !== "darwin") throw new Error("--sf-trak requires macOS");
    const bytes = await readFile("/System/Library/Fonts/SFNS.ttf");
    if (!tableTags(bytes).includes("trak")) throw new Error("Explicit SF font has no trak table");
    fixtures.push({ id: "sf-probe", family: "SF byte probe", weight: 400, style: "normal", bytes,
      expected: sha256(bytes), sourceURL: "system:SFNS.ttf", license: "Apple proprietary; local probe only; do not redistribute" });
  }
  const fonts = [];
  for (const f of fixtures) {
    const hash = sha256(f.bytes);
    if (hash !== f.expected) throw new Error(`Fixture SHA mismatch: ${f.id}`);
    const sourceBytes = `fonts/${f.id}.ttf`;
    await writeFile(new URL(sourceBytes, OUT), f.bytes);
    const license = { name: f.license, sourceURL: f.licenseURL ?? null, sha256: null, file: null };
    if (f.licenseBytes) {
      license.sha256 = sha256(f.licenseBytes); license.file = `fonts/${f.id}-license.txt`;
      await writeFile(new URL(license.file, OUT), f.licenseBytes);
    }
    fonts.push({ manifest: { id: f.id, family: f.family, hash, weight: f.weight, style: f.style, url: `/${sourceBytes}` },
      privateFamily: `oc_${hash}_${f.weight}_${f.style}`, sourceURL: f.sourceURL, sourceBytes,
      byteSize: f.bytes.length, tableTags: tableTags(f.bytes), license });
  }
  return fonts;
}
