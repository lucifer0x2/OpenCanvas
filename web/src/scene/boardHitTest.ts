import { hitTest } from "../canvasHitTest";
import type { FrameSceneView } from "../hooks/useScene";
import type { FramePlacement } from "./frameLayout";

/** Translate only the pointer; the scene and its inspector bounds stay frame-local. */
export function boardHitTest(
  frames: FrameSceneView[],
  placements: FramePlacement[],
  x: number,
  y: number,
  revision?: number,
) {
  for (let i = frames.length - 1; i >= 0; i--) {
    const frame = frames[i],
      offset = placements[i];
    if (
      frame.scene &&
      (revision === undefined || frame.scene.provenance.revision === revision)
    ) {
      const node = hitTest(frame.scene, x - offset.x, y - offset.y);
      if (node) return node;
    }
  }
}
