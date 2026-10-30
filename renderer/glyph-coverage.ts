/** Checked Unicode cmap (SFNT format4/12) coverage. Font readiness is not coverage.
 * WOFF/WOFF2 and unsupported cmap formats return undefined, never a fabricated pass.
 * This proves scalar mappings, not cluster shaping/variation/emoji sequence support.
 */
export type GlyphCoverage = (codePoint: number) => boolean;
export function parseCoverage(bytes: ArrayBuffer): GlyphCoverage | undefined {
  const view = new DataView(bytes);
  const inside = (at: number, length: number) => Number.isInteger(at) && at >= 0 && length >= 0 && at + length <= view.byteLength;
  const u16 = (at: number) => view.getUint16(at);
  const u32 = (at: number) => view.getUint32(at);
  if (!inside(0, 12) || ![0x00010000, 0x4f54544f, 0x74727565].includes(u32(0))) return;
  const count = u16(4);
  if (!inside(12, count * 16)) return;
  let cmap = -1, cmapLength = 0;
  for (let n = 0; n < count; n++) {
    const at = 12 + n * 16;
    if (u32(at) === 0x636d6170) { cmap = u32(at + 8); cmapLength = u32(at + 12); }
  }
  if (!inside(cmap, cmapLength) || cmapLength < 4) return;
  const tables = u16(cmap + 2);
  if (4 + tables * 8 > cmapLength) return;
  const checks: GlyphCoverage[] = [];
  for (let n = 0; n < tables; n++) {
    const record = cmap + 4 + n * 8;
    const platform = u16(record), encoding = u16(record + 2);
    if (!(platform === 0 || (platform === 3 && [1, 10].includes(encoding)))) continue;
    const at = cmap + u32(record + 4);
    if (!inside(at, 4) || at < cmap || at + 4 > cmap + cmapLength) continue;
    const format = u16(at);
    if (format === 12 && inside(at, 16)) {
      const length = u32(at + 4), groups = u32(at + 12);
      if (length < 16 || !inside(at, length) || at + length > cmap + cmapLength || groups > (length - 16) / 12) continue;
      let previous = -1, valid = true;
      for (let i = 0; i < groups; i++) {
        const group = at + 16 + i * 12, start = u32(group), end = u32(group + 4);
        if (start > end || start <= previous || end > 0x10ffff) { valid = false; break; }
        previous = end;
      }
      if (!valid) continue;
      checks.push((cp) => {
        let low = 0, high = groups - 1;
        while (low <= high) {
          const mid = (low + high) >>> 1, group = at + 16 + mid * 12;
          if (cp < u32(group)) high = mid - 1;
          else if (cp > u32(group + 4)) low = mid + 1;
          else return u32(group + 8) + cp - u32(group) !== 0;
        }
        return false;
      });
    } else if (format === 4 && inside(at, 16)) {
      const length = u16(at + 2), segments = u16(at + 6) / 2;
      if (!Number.isInteger(segments) || segments < 1 || length < 16 + segments * 8 || !inside(at, length) || at + length > cmap + cmapLength) continue;
      const ends = at + 14, starts = ends + segments * 2 + 2;
      const deltas = starts + segments * 2, offsets = deltas + segments * 2;
      let previous = -1, valid = true;
      for (let i = 0; i < segments; i++) {
        if (u16(starts + i * 2) > u16(ends + i * 2) || u16(ends + i * 2) <= previous) { valid = false; break; }
        previous = u16(ends + i * 2);
      }
      if (!valid) continue;
      checks.push((cp) => {
        if (cp > 0xffff) return false;
        let low = 0, high = segments - 1;
        while (low < high) {
          const mid = (low + high) >>> 1;
          if (u16(ends + mid * 2) < cp) low = mid + 1; else high = mid;
        }
        if (cp < u16(starts + low * 2) || cp > u16(ends + low * 2)) return false;
        const delta = u16(deltas + low * 2), offset = u16(offsets + low * 2);
        if (!offset) return ((cp + delta) & 0xffff) !== 0;
        const glyphAt = offsets + low * 2 + offset + 2 * (cp - u16(starts + low * 2));
        if (glyphAt < at || glyphAt + 2 > at + length) return false;
        const glyph = u16(glyphAt);
        return glyph !== 0 && ((glyph + delta) & 0xffff) !== 0;
      });
    }
  }
  return checks.length ? (cp) => checks.some((check) => check(cp)) : undefined;
}

/** NFC composition for coverage only avoids accusing a decomposed Vietnamese cluster
 * when the font covers its canonical composition. Source and rendered text stay unchanged.
 */
export function missingCodePoints(text: string, coverage: GlyphCoverage): number[] {
  const missing = new Set<number>();
  const scalars = (s: string) => [...s].filter(char => !/[\p{White_Space}\p{Default_Ignorable_Code_Point}\p{Cc}]/u.test(char)).map(char => char.codePointAt(0)!);
  for (const { segment } of new Intl.Segmenter("und", { granularity: "grapheme" }).segment(text)) {
    const raw = scalars(segment), canonical = scalars(segment.normalize("NFC"));
    if (raw.every(coverage) || canonical.every(coverage)) continue;
    for (const cp of canonical) if (!coverage(cp)) missing.add(cp);
  }
  return [...missing].sort((a, b) => a - b);
}
