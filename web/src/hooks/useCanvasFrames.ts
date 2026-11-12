import { useCallback, useMemo } from "react";
import { fitFrameBoard, type FrameSize } from "../scene/frameLayout";
import type { SceneView } from "./useScene";

type Size = { width: number; height: number };

/** Scene publication changes paint inputs, but only frame geometry changes board packing. */
export function useCanvasFrames(view: SceneView, frameName: string) {
  const { scene, resources } = view;
  const frames = useMemo(
    () => view.frames ?? (scene && resources
      ? [{
          id: scene.provenance.frameId,
          name: frameName,
          ...scene.provenance.viewport,
          scene,
          resources,
          revision: scene.provenance.revision,
        }]
      : []),
    [view.frames, scene, resources, frameName],
  );
  const geometry = useMemo(
    () => JSON.stringify(frames.map(({ id, width, height }) => ({ id, width, height }))),
    [frames],
  );
  const boardLayout = useMemo(() => {
    // Decode our own serialization so this memo depends only on authored geometry,
    // never on progressive scene/resource objects or their revision.
    const sizes: FrameSize[] = JSON.parse(geometry);
    let cached: { size: Size; layout: ReturnType<typeof fitFrameBoard> } | undefined;
    return (size: Size) => {
      if (cached?.size.width === size.width && cached.size.height === size.height)
        return cached.layout;
      const layout = fitFrameBoard(sizes, size);
      cached = { size, layout };
      return layout;
    };
  }, [geometry]);
  const all = Boolean(view.frames);
  const viewport = scene?.provenance.viewport;
  const content = useCallback(
    (size: Size) => all ? frames.length ? boardLayout(size) : undefined : viewport,
    [all, frames.length, boardLayout, viewport],
  );
  return { frames, boardLayout, content };
}
