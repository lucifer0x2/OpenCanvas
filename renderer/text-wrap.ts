/** pre-line whitespace with explicit LF and grapheme-safe emergency breaks.
 * NBSP, narrow NBSP and word joiner glue their entire word: even undersized boxes
 * preserve these nonbreaking groups. We do not claim full Unicode line breaking/bidi.
 */
export function wrapText(text: string, fits: (text: string) => boolean): string[] {
  const lines: string[] = [];
  const graphemes = new Intl.Segmenter("und", { granularity: "grapheme" });
  for (const paragraph of text.replace(/\r\n?/g, "\n").split("\n")) {
    const words = paragraph.replace(/^[\t\f\v ]+|[\t\f\v ]+$/g, "").split(/[\t\f\v ]+/u).filter(Boolean);
    let line = "";
    for (const word of words) {
      const next = line ? `${line} ${word}` : word;
      if (fits(next)) { line = next; continue; }
      if (line) { lines.push(line); line = ""; }
      if (fits(word) || /[\u00a0\u202f\u2060\ufeff]/u.test(word)) { line = word; continue; }
      for (const { segment } of graphemes.segment(word)) {
        if (line && !fits(line + segment)) { lines.push(line); line = ""; }
        line += segment;
      }
    }
    lines.push(line);
  }
  return lines;
}
