import { memo } from "react";
import type { FrameSceneView } from "../hooks/useScene";
import type { BoardPaint } from "../paint/boardPaint";
import type { FramePlacement } from "../scene/frameLayout";
import { CanvasFrame } from "./CanvasFrame";
import { sceneNodes } from "../scene/sceneNodes";

export const CanvasScenes = memo(function CanvasScenes({
  frames,
  placements,
  scale,
  selectedId,
  labels,
  board,
}: {
  frames: FrameSceneView[];
  placements: FramePlacement[];
  scale: number;
  selectedId: string;
  labels: boolean;
  board?: BoardPaint;
}) {
  return frames.map((frame, index) => (
    <CanvasFrame
      key={frame.id}
      frame={frame}
      placement={placements[index]}
      scale={scale}
      selectedId={
        selectedId && frame.scene && sceneNodes(frame.scene).has(selectedId)
          ? selectedId
          : ""
      }
      labels={labels}
      board={board}
    />
  ));
});
