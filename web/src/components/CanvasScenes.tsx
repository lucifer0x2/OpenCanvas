import type { FrameSceneView } from "../hooks/useScene";
import type { FramePlacement } from "../scene/frameLayout";
import { CanvasFrame } from "./CanvasFrame";

export function CanvasScenes({
  frames,
  placements,
  scale,
  selectedId,
  labels,
}: {
  frames: FrameSceneView[];
  placements: FramePlacement[];
  scale: number;
  selectedId: string;
  labels: boolean;
}) {
  return frames.map((frame, index) => (
    <CanvasFrame
      key={frame.id}
      frame={frame}
      placement={placements[index]}
      scale={scale}
      selectedId={selectedId}
      labels={labels}
    />
  ));
}
