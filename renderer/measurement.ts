import { type Measurement, type MeasureRequest, type Resources } from "./types";
import { PipelineError } from "./error";
import { fontCSS } from "./font-family";
import { wrapText } from "./text-wrap";
import { transformText } from "./text-transform";
import { missingCodePoints } from "./glyph-coverage";
const round = (value: number) => Math.round(value * 64) / 64;
const ceil64 = (value: number) => Math.ceil(value * 64) / 64;

/** One implementation of whitespace wrapping for Rust measurement and all exported line lists.
 * LF forces a line; runs of horizontal whitespace collapse. Long words split on graphemes.
 */
export function measureBatch(
  requests: MeasureRequest[],
  resources: Resources,
): Record<string, Measurement> {
  const context = document.createElement("canvas").getContext("2d");
  if (!context) throw new PipelineError("CANVAS_UNAVAILABLE", "Canvas2D unavailable");
  if (!("letterSpacing" in context)) throw new PipelineError("TEXT_CAPABILITY_UNAVAILABLE", "Canvas letterSpacing is required by this renderer");
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
    context.letterSpacing = `${r.letterSpacing ?? 0}px`;
    const metric = context.measureText("");
    const ascent = metric.fontBoundingBoxAscent, descent = metric.fontBoundingBoxDescent;
    if (!Number.isFinite(ascent) || !Number.isFinite(descent)) throw new PipelineError("TEXT_CAPABILITY_UNAVAILABLE", "Font bounding metrics are required");
    const baseline = ascent + Math.floor((r.lineHeight - ascent - descent) / 2);
    const maxWidth = r.width === null ? Infinity : Math.max(0, r.width);
    // Fit on the same 1/64 grid as the reported width, so text laid out at its own
    // measured max-content width does not wrap again.
    const width = (text: string) => ceil64(Math.max(0, context.measureText(text).width));
    const rendered = transformText(r.text, r.textTransform);
    const lines = wrapText(rendered, (text) => width(text) <= maxWidth).map((text, index) => {
      const ink = context.measureText(text);
      return { text, x: 0, y: round(index * r.lineHeight + baseline), width: width(text),
        inkBounds: { x: -ink.actualBoundingBoxLeft, y: -ink.actualBoundingBoxAscent,
          width: ink.actualBoundingBoxLeft + ink.actualBoundingBoxRight,
          height: ink.actualBoundingBoxAscent + ink.actualBoundingBoxDescent } };
    });
    const coverage = resources.glyphCoverage?.get(font.id);
    output[r.key] = {
      width: Math.max(0, ...lines.map((l) => l.width)),
      height: round(lines.length * r.lineHeight),
      lines,
      ascent,
      missing: coverage ? missingCodePoints(rendered, coverage) : [],
      coverageUnavailable: !coverage,
    };
  }
  return output;
}
