import { useEffect, useMemo, useState } from "react";
import { MipChain } from "../mipmap";
import type { Raster, SceneView } from "./useScene";

// Re-rasterize once a zoom gesture settles; the previous bitmap covers the gap.
const RASTER_DELAY = 140;
const sameScale = (a: number, b: number) => Math.abs(Math.log2(a / b)) < 0.05;

/**
 * The bitmap to display at `scale` CSS units per scene unit. Scaling one 1:1 rasterization to
 * every zoom level softens text, so the scene is drawn again at the size it is actually shown
 * once the gesture settles; until then a half-size copy previews it without speckling.
 */
export function useDisplayRaster(
  view: SceneView,
  scale: number,
  pixelRatio: number,
): HTMLCanvasElement | undefined {
  const wanted = scale * pixelRatio;
  const mips = useMemo(
    () => (view.canvas ? new MipChain(view.canvas) : null),
    [view.canvas],
  );
  const [raster, setRaster] = useState<(Raster & { requested: number }) | null>(
    null,
  );
  useEffect(() => setRaster(null), [view.canvas]);
  useEffect(() => {
    const renderAt = view.renderAt;
    if (!renderAt || (raster && sameScale(raster.requested, wanted))) return;
    const timer = setTimeout(() => {
      const next = renderAt(wanted);
      if (next) setRaster({ ...next, requested: wanted });
    }, RASTER_DELAY);
    return () => clearTimeout(timer);
  }, [view.renderAt, wanted, raster]);

  // Match on what was requested, not on what the renderer could deliver: past its scale cap the
  // capped raster is still far sharper than scaling the 1:1 canvas back up.
  if (raster && sameScale(raster.requested, wanted)) return raster.canvas;
  const viewport = view.scene?.provenance.viewport;
  return mips && viewport ? mips.pick(viewport.width * wanted) : undefined;
}
