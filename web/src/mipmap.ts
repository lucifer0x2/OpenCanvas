/**
 * Half-size copies of a rendered scene so a zoomed-out canvas samples a bitmap near its drawn
 * size. Canvas2D downscales in a single bilinear step and speckles text at arbitrary ratios;
 * `imageSmoothingQuality` does not change that, while halving in steps resolves cleanly.
 */
export class MipChain {
  private levels: HTMLCanvasElement[];

  constructor(base: HTMLCanvasElement) {
    this.levels = [base];
  }

  /** The level closest to `width` device pixels. */
  pick(width: number): HTMLCanvasElement {
    const base = this.levels[0];
    if (!Number.isFinite(width) || width <= 0 || width >= base.width)
      return base;
    // Nearest level, not the next larger one: measured against an exact area average, a level
    // slightly below the drawn size stays more faithful than a 1.9x single-step downscale.
    const wanted = Math.round(Math.log2(base.width / width));
    while (this.levels.length <= wanted) {
      const half = halve(this.levels[this.levels.length - 1]);
      if (!half) break;
      this.levels.push(half);
    }
    return this.levels[Math.min(wanted, this.levels.length - 1)];
  }
}

function halve(source: HTMLCanvasElement): HTMLCanvasElement | null {
  const width = Math.floor(source.width / 2);
  const height = Math.floor(source.height / 2);
  if (width < 1 || height < 1) return null;
  const target = document.createElement("canvas");
  target.width = width;
  target.height = height;
  const context = target.getContext("2d");
  if (!context) return null;
  context.imageSmoothingEnabled = true;
  context.drawImage(source, 0, 0, width, height);
  return target;
}
