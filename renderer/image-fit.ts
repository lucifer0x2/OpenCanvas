/** Image `fit` values, the same list the storage validator accepts (crates/storage/src/image_fit.rs). */
export const IMAGE_FITS = ["fit", "contain", "cover"] as const;
export type ImageFit = (typeof IMAGE_FITS)[number];

/**
 * Size and offset of the drawn image inside its bounds. `fit` stretches, `contain` scales the
 * whole source inside, `cover` scales it to fill (the clip crops the rest). `fill` is the scene
 * name of `fit` before the names were unified and may still arrive in a cached scene.
 */
export function placeImage(
  fit: string,
  box: { width: number; height: number },
  source: { width: number; height: number },
): { x: number; y: number; width: number; height: number } {
  let width = box.width,
    height = box.height;
  if (fit !== "fit" && fit !== "fill") {
    const scale = fit === "cover" ? Math.max : Math.min;
    const ratio = scale(box.width / source.width, box.height / source.height);
    width = source.width * ratio;
    height = source.height * ratio;
  }
  return {
    x: (box.width - width) / 2,
    y: (box.height - height) / 2,
    width,
    height,
  };
}
