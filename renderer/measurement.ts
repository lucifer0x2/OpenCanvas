import {
  type FontManifest,
  type Measurement,
  type MeasureRequest,
  type Resources,
  type TextLine,
} from "./types";
import { PipelineError } from "./error";

const canvas = () => {
  const context = document.createElement("canvas").getContext("2d");
  if (!context)
    throw new PipelineError("CANVAS_UNAVAILABLE", "Canvas2D unavailable");
  return context;
};
const fontCSS = (font: FontManifest, size: number) =>
  `${font.style} ${font.weight} ${size}px "${font.family}"`;
const round = (value: number) => Math.round(value * 64) / 64;

/** One implementation of whitespace wrapping for Rust measurement and all exported line lists.
 * LF forces a line; runs of horizontal whitespace collapse. Long words split on graphemes.
 */
export function measureBatch(
  requests: MeasureRequest[],
  resources: Resources,
): Record<string, Measurement> {
  const context = canvas();
  const output: Record<string, Measurement> = {};
  for (const r of requests) {
    const font = resources.fonts.get(r.fontId);
    if (
      !font ||
      font.hash !== r.fontHash ||
      font.family !== r.family ||
      font.weight !== r.weight ||
      font.style !== r.style
    )
      throw new PipelineError(
        "FONT_MISSING",
        `Exact font ${r.fontId} is not ready`,
      );
    context.font = fontCSS(font, r.fontSize);
    context.fontKerning = "normal";
    const maxWidth = r.width === null ? Infinity : Math.max(0, r.width);
    // Fit on the same 1/64 grid as the reported width, so text laid out at its own
    // measured max-content width does not wrap again.
    const fits = (text: string) =>
      round(context.measureText(text).width) <= maxWidth;
    const lines: TextLine[] = [];
    const add = (text: string) =>
      lines.push({
        text,
        x: 0,
        y: round(
          lines.length * r.lineHeight +
            r.fontSize +
            (r.lineHeight - r.fontSize) / 2,
        ),
        width: round(context.measureText(text).width),
      });
    for (const paragraph of r.text.replace(/\r\n?/g, "\n").split("\n")) {
      const words = paragraph
        .trim()
        .split(/[^\S\n]+/u)
        .filter(Boolean);
      let line = "";
      for (const word of words) {
        const next = line ? `${line} ${word}` : word;
        if (fits(next)) {
          line = next;
          continue;
        }
        if (line) {
          add(line);
          line = "";
        }
        if (fits(word)) {
          line = word;
          continue;
        }
        const parts = new Intl.Segmenter("vi", {
          granularity: "grapheme",
        }).segment(word);
        for (const part of parts) {
          if (
            line &&
            !fits(line + part.segment)
          ) {
            add(line);
            line = "";
          }
          line += part.segment;
        }
      }
      add(line);
    }
    output[r.key] = {
      width: Math.max(0, ...lines.map((l) => l.width)),
      height: round(lines.length * r.lineHeight),
      lines,
    };
  }
  return output;
}
