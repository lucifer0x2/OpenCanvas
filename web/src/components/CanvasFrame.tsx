import { memo, useCallback, useMemo } from "react";
import { Group, Rect, Shape, Text } from "react-konva";
import type Konva from "konva";
import { indexScene, paintScene } from "../../../renderer/index";
import type { FrameSceneView } from "../hooks/useScene";
import type { FramePlacement } from "../scene/frameLayout";
import { frameLabelMetrics } from "../scene/frameLabels";

/** Stable ready frames keep their React nodes and paint callback when the next slot completes. */
export const CanvasFrame = memo(function CanvasFrame({
  frame,
  placement,
  scale,
  selectedId,
  labels,
}: {
  frame: FrameSceneView;
  placement: FramePlacement;
  scale: number;
  selectedId: string;
  labels: boolean;
}) {
  const selected = frame.scene?.nodes.find((node) => node.id === selectedId);
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
  const paint = useCallback(
    (context: Konva.Context) => {
      if (!frame.scene || !frame.resources) return;
      context._context.imageSmoothingQuality = "high";
      paintScene(context._context, frame.scene, frame.resources, tree);
    },
    [frame.scene, frame.resources, tree],
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
