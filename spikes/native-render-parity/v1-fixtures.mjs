// Reuse G1's original bytes and licenses; no downloads or installed family lookup.
import { readFile } from "node:fs/promises";
import { OUT, sha256 } from "./environment.mjs";

export const INTER_HASH = "4989b125924991b90d05b2d16e0e388c48f7d5bb8b30539bbf9c755278d0ccaf";
export const WEIGHTS = [100, 400, 500, 700, 900];

export function headerFacts(bytes) {
  const tables = new Map(Array.from({ length: bytes.readUInt16BE(4) }, (_, i) => {
    const at = 12 + i * 16;
    const start = bytes.readUInt32BE(at + 8), size = bytes.readUInt32BE(at + 12);
    return [bytes.toString("ascii", at, at + 4), bytes.subarray(start, start + size)];
  }));
  const fvar = tables.get("fvar"), avar = tables.get("avar"), mvar = tables.get("MVAR");
  const axes = fvar ? Array.from({ length: fvar.readUInt16BE(8) }, (_, i) => {
    const at = fvar.readUInt16BE(4) + i * fvar.readUInt16BE(10);
    return { tag: fvar.toString("ascii", at, at + 4), min: fvar.readInt32BE(at + 4) / 65536,
      default: fvar.readInt32BE(at + 8) / 65536, max: fvar.readInt32BE(at + 12) / 65536 };
  }) : [];
  let cursor = 8;
  const maps = avar ? Array.from({ length: avar.readUInt16BE(6) }, () => {
    const count = avar.readUInt16BE(cursor); cursor += 2;
    return Array.from({ length: count }, () => {
      const pair = [avar.readInt16BE(cursor) / 16384, avar.readInt16BE(cursor + 2) / 16384];
      cursor += 4; return pair;
    });
  }) : [];
  const hhea = tables.get("hhea"), os2 = tables.get("OS/2"), head = tables.get("head");
  return { tables: [...tables.keys()].sort(), axes,
    avar: avar ? { version: `${avar.readUInt16BE(0)}.${avar.readUInt16BE(2)}`, maps } : null,
    mvarTags: mvar ? Array.from({ length: mvar.readUInt16BE(8) }, (_, i) => {
      const at = 12 + i * mvar.readUInt16BE(6); return mvar.toString("ascii", at, at + 4);
    }) : [],
    hhea: { ascent: hhea.readInt16BE(4), descent: hhea.readInt16BE(6), lineGap: hhea.readInt16BE(8) },
    os2: { weight: os2.readUInt16BE(4), fsSelection: os2.readUInt16BE(62) },
    head: { unitsPerEm: head.readUInt16BE(18), macStyle: head.readUInt16BE(44) } };
}

export async function variableFixtures(source) {
  const fonts = [];
  const add = async (originalId, id, weight, style = "normal") => {
    const base = source.fonts.find((f) => f.manifest.id === originalId);
    if (!base) throw new Error(`Missing G1 fixture ${originalId}`);
    const bytes = await readFile(new URL(base.sourceBytes, OUT));
    if (sha256(bytes) !== base.manifest.hash) throw new Error(`Source SHA mismatch: ${originalId}`);
    if (originalId === "inter" && base.manifest.hash !== INTER_HASH) throw new Error("Inter pin mismatch");
    fonts.push({ ...base, manifest: { ...base.manifest, id, weight, style },
      privateFamily: `oc_${base.manifest.hash}_${weight}_${style}`, headerFacts: headerFacts(bytes) });
  };
  for (const weight of WEIGHTS) {
    await add("inter", `v1-inter-${weight}`, weight);
    await add("noto", `v1-noto-${weight}`, weight);
  }
  // These bytes have no ital/slnt axes: descriptors must not manufacture outlines.
  await add("inter", "v1-inter-400-italic", 400, "italic");
  await add("inter", "v1-inter-700-italic", 700, "italic");
  await add("lato-regular", "v1-static-regular", 400);
  await add("lato-bold", "v1-static-bold", 700);
  await add("lato-italic", "v1-static-italic", 400, "italic");
  await add("lato-regular", "v1-static-alias", 700, "italic");
  return fonts;
}

export function variableCases(fonts) {
  const requests = [], cases = {};
  for (const { manifest: m } of fonts) for (const fontSize of [16, 24, 32]) {
    for (const letterSpacing of [0, -2, 0.5]) {
      for (const [label, text] of Object.entries({ root: "AVATAR office Tiếng Việt",
        kern: "AV To office ffi", vietnamese: "Tiếng Việt ắ Nguyễn" })) {
        const key = `current/v1/${m.id}/${fontSize}/${letterSpacing}/${label}`;
        requests.push({ key, text, fontId: m.id, fontHash: m.hash, family: m.family,
          weight: m.weight, style: m.style, fontSize, lineHeight: fontSize * 1.375,
          letterSpacing, textTransform: "none", width: null });
        cases[key] = { group: "variable-instance", label, declaredFallback: false, thresholdOf: null };
      }
    }
  }
  return { requests, cases };
}
