import { memo, useCallback, useEffect, useMemo } from "react";
import { Group, Rect, Shape, Text } from "react-konva";
import type Konva from "konva";
import { indexScene, paintScene } from "../../../renderer/index";
import type { FrameSceneView } from "../hooks/useScene";
import type { FramePlacement } from "../scene/frameLayout";
import { frameLabelMetrics } from "../scene/frameLabels";
import type { BoardPaint } from "../paint/boardPaint";
import { FrameBitmap } from "../paint/frameBitmap";
import { sceneNodes } from "../scene/sceneNodes";

/** Stable ready frames keep their React nodes and paint callback when the next slot completes. */
export const CanvasFrame = memo(function CanvasFrame({
  frame,
  placement,
  scale,
  selectedId,
  labels,
  board,
}: {
  frame: FrameSceneView;
  placement: FramePlacement;
  scale: number;
  selectedId: string;
  labels: boolean;
  /** Board bitmap cache; without it (single frame) the scene paints as vectors. */
  board?: BoardPaint;
}) {
  const selected = frame.scene && selectedId
    ? sceneNodes(frame.scene).get(selectedId)
    : undefined;
  const label = frameLabelMetrics(
    frame.width,
    scale,
    placement.labelSpace ?? 32,
  );
  // Resource maps are shared across progressive slots; the ready frame's resource
  // object is the boundary, including a new object when failed resources are retried.
  const tree = useMemo(
    () =>
      frame.scene && frame.resources
        ? indexScene(frame.scene, frame.resources)
        : undefined,
    [frame.scene, frame.resources],
  );
  // One bitmap per mounted frame, released on unmount; it re-rasterizes itself when the
  // scene, resources or resting zoom differ from what it holds.
  const bitmap = useMemo(() => board && new FrameBitmap(board), [board]);
  useEffect(() => () => bitmap?.release(), [bitmap]);
  // A slot back to its skeleton (Retry, new scope) draws no Shape, so free its pixels here.
  useEffect(() => {
    if (!frame.scene) bitmap?.release();
  }, [bitmap, frame.scene]);
  const paint = useCallback(
    (context: Konva.Context) => {
      if (!frame.scene || !frame.resources || !tree) return;
      const content = { scene: frame.scene, resources: frame.resources, tree };
      const vector = (target: CanvasRenderingContext2D) => {
        target.imageSmoothingQuality = "high";
        paintScene(target, content.scene, content.resources, content.tree);
      };
      if (bitmap) bitmap.draw(context._context, content, vector);
      else vector(context._context);
    },
    [frame.scene, frame.resources, tree, bitmap],
  );
  const pending = !frame.scene && !frame.error;
  return (
    <Group x={placement.x} y={placement.y} name="canvas-frame" id={frame.id}>
      {labels && label.visible && (
        <Text
          text={frame.name}
          x={0}
          y={label.y}
          width={frame.width}
          height={label.height}
          fontSize={label.fontSize}
          fontFamily="Arial, sans-serif"
          fill="#505b6b"
          ellipsis
          wrap="none"
          listening={false}
        />
      )}
      <Rect
        width={frame.width}
        height={frame.height}
        fill={pending ? "#f5f7fa" : "white"}
        stroke={pending ? "#d9dfe8" : undefined}
        strokeWidth={pending ? 1 / scale : 0}
        shadowColor="#1d2433"
        shadowBlur={pending ? 0 : 20}
        shadowOffsetY={5}
        shadowOpacity={0.12}
      />
      {pending &&
        [0.65, 0.85, 0.5].map((width, index) => (
          <Rect
            key={index}
            x={frame.width * 0.1}
            y={frame.height * (0.12 + index * 0.1)}
            width={frame.width * width}
            height={frame.height * 0.035}
            fill="#e4e9f0"
            listening={false}
          />
        ))}
      {frame.scene && frame.resources && (
        <Shape listening={false} sceneFunc={paint} />
      )}
      {frame.error && (
        <Text
          text={`Frame unavailable\n${frame.error}`}
          x={24}
          y={24}
          width={Math.max(1, frame.width - 48)}
          fontSize={16}
          lineHeight={1.5}
          fill="#a73429"
          listening={false}
        />
      )}
      {selected && (
        <Rect
          {...selected.bounds}
          stroke="#1672e8"
          strokeWidth={1.5 / scale}
          listening={false}
        />
      )}
    </Group>
  );
});
