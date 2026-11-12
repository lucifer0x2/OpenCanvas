import { BOARD_BITMAP_PIXELS, sameScale } from "./bitmapBox";

/** A scale must hold this long before frames re-rasterize; gestures paint vectors meanwhile. */
export const ZOOM_SETTLE_MS = 180;

type Timer = ReturnType<typeof setTimeout>;

/**
 * State shared by one board's frame bitmaps: whether the zoom is at rest and the pixel budget.
 * `redraw` asks the layer for another draw once a zoom settles, so bitmaps catch up.
 */
export class BoardPaint {
  private scale: number | undefined;
  private changedAt = -Infinity;
  private timer: Timer | undefined;
  private pixels = 0;

  constructor(
    private readonly redraw: () => void,
    private readonly now: () => number = () => performance.now(),
    private readonly budget = BOARD_BITMAP_PIXELS,
  ) {}

  /** Notes this draw's device scale; true when it has held for ZOOM_SETTLE_MS. */
  settled(scale: number) {
    // The first scale a board paints at (its fitted camera) counts as at rest.
    if (this.scale === undefined) this.scale = scale;
    else if (!sameScale(scale, this.scale)) {
      this.scale = scale;
      this.changedAt = this.now();
      clearTimeout(this.timer);
      this.timer = setTimeout(() => {
        this.timer = undefined;
        this.changedAt = -Infinity;
        this.redraw();
      }, ZOOM_SETTLE_MS);
    }
    return this.now() - this.changedAt >= ZOOM_SETTLE_MS;
  }

  /** Claims bitmap pixels; false over the board budget (the frame paints vectors instead). */
  reserve(pixels: number) {
    if (this.pixels + pixels > this.budget) return false;
    this.pixels += pixels;
    return true;
  }

  release(pixels: number) {
    this.pixels = Math.max(0, this.pixels - pixels);
  }

  /** Live bitmap pixels, for budget decisions and diagnostics. */
  get livePixels() {
    return this.pixels;
  }

  dispose() {
    clearTimeout(this.timer);
    this.timer = undefined;
  }
}
