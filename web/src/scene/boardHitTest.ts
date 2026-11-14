import { hitTest } from "../canvasHitTest";
import type { FrameSceneView } from "../hooks/useScene";
import type { FramePlacement } from "./frameLayout";
import { currentScene } from "./staleFrames";

/**
 * Translate only the pointer; the scene and its inspector bounds stay frame-local. Only
 * scenes current at `revision` are hit; stale ones are skipped until revalidated.
 */
export function boardHitTest(
  frames: FrameSceneView[],
  placements: FramePlacement[],
  x: number,
  y: number,
  revision: number | undefined,
) {
  for (let i = frames.length - 1; i >= 0; i--) {
    const scene = currentScene(frames[i], revision),
      offset = placements[i];
    if (scene) {
      const node = hitTest(scene, x - offset.x, y - offset.y);
      if (node) return node;
    }
  }
}
