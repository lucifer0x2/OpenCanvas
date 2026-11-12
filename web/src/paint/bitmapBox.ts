import type { LogicalRect } from "../../../renderer/index";

/** One bitmap stays under Safari's 16,777,216-pixel canvas area and 16,384-pixel side. */
export const MAX_BITMAP_PIXELS = 16_000_000;
export const MAX_BITMAP_SIDE = 16_384;
/** All live frame bitmaps of a board: 32 M pixels, 128 MiB of RGBA. */
export const BOARD_BITMAP_PIXELS = 32 * 1024 * 1024;

/** The parts of a device transform a frame bitmap depends on: uniform scale, no rotation. */
export interface DeviceTransform {
  a: number;
  b: number;
  c: number;
  d: number;
  e: number;
  f: number;
}

/**
 * A bitmap's device-pixel box: whole pixels around `extent` (scene units) at scale `m.a`,
 * keeping the frame origin's subpixel phase so a blit at a whole-pixel offset lands exactly
 * where vector paint would. Same one-pixel antialiasing margin as renderer opacity layers.
 */
export interface BitmapBox {
  scale: number;
  phaseX: number;
  phaseY: number;
  x0: number;
  y0: number;
  width: number;
  height: number;
}

/**
 * `undefined` when the transform rotates/skews or the bitmap would exceed canvas limits or
 * `maxPixels` (the board passes its own canvas area: see FrameBitmap).
 */
export function bitmapBox(
  extent: LogicalRect,
  m: DeviceTransform,
  maxPixels = MAX_BITMAP_PIXELS,
): BitmapBox | undefined {
  const scale = m.a;
  if (m.b || m.c || !(scale > 0) || Math.abs(m.d - scale) > scale * 1e-9) return;
  const phaseX = m.e - Math.floor(m.e),
    phaseY = m.f - Math.floor(m.f);
  const x0 = Math.floor(phaseX + extent.x0 * scale) - 1,
    y0 = Math.floor(phaseY + extent.y0 * scale) - 1;
  const width = Math.ceil(phaseX + extent.x1 * scale) + 1 - x0,
    height = Math.ceil(phaseY + extent.y1 * scale) + 1 - y0;
  if (
    !Number.isFinite(width + height) ||
    width > MAX_BITMAP_SIDE ||
    height > MAX_BITMAP_SIDE ||
    width * height > Math.min(maxPixels, MAX_BITMAP_PIXELS)
  )
    return;
  return { scale, phaseX, phaseY, x0, y0, width, height };
}

/** Device position of the box under `m`; exact when `m` keeps the box's phase (pan). */
export function blitOrigin(box: BitmapBox, m: DeviceTransform) {
  return {
    x: Math.round(m.e - box.phaseX) + box.x0,
    y: Math.round(m.f - box.phaseY) + box.y0,
  };
}

/** Same scale within float noise: a pan keeps the bitmap, any zoom does not. */
export const sameScale = (a: number, b: number) =>
  Math.abs(a - b) <= Math.max(a, b) * 1e-9;

/**
 * Where `extent` lands on a `width`×`height` target under `m`: "visible", "near" (within one
 * target size; keep any bitmap, paint nothing) or "far" (release the bitmap).
 */
export function placement(
  extent: LogicalRect,
  m: DeviceTransform,
  width: number,
  height: number,
): "visible" | "near" | "far" {
  const x0 = m.e + extent.x0 * m.a,
    x1 = m.e + extent.x1 * m.a,
    y0 = m.f + extent.y0 * m.d,
    y1 = m.f + extent.y1 * m.d;
  if (x1 > 0 && y1 > 0 && x0 < width && y0 < height) return "visible";
  return x1 > -width && y1 > -height && x0 < 2 * width && y0 < 2 * height
    ? "near"
    : "far";
}
