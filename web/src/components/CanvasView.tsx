import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { Stage, Layer } from "react-konva";
import type Konva from "konva";
import { BoardPaint } from "../paint/boardPaint";
import { boardHitTest } from "../scene/boardHitTest";
import { fitFrameBoard } from "../scene/frameLayout";
import { CanvasScenes } from "./CanvasScenes";
import { CanvasChrome } from "./CanvasChrome";
import { CanvasMessage } from "./CanvasMessage";
import {
  useCanvasTransform,
  wheelZoomFactor,
} from "../hooks/useCanvasTransform";
import type { SceneView } from "../hooks/useScene";
import { useNodeActions } from "./NodeActionsProvider";

// A click that moves further than this is a pan, not a selection.
const CLICK_SLOP = 5;

export function CanvasView({
  view,
  selectedId,
  select,
  retry,
  frameName,
}: {
  view: SceneView;
  selectedId: string;
  select: (id: string) => void;
  retry: () => void;
  frameName: string;
}) {
  const stage = useRef<Konva.Stage>(null);
  const layer = useRef<Konva.Layer>(null);
  // Frame bitmaps of the All board share zoom-settle and memory state; one per canvas.
  const [board] = useState(
    () => new BoardPaint(() => layer.current?.batchDraw()),
  );
  useEffect(() => () => board.dispose(), [board]);
  const actions = useNodeActions();
  const pointerStart = useRef({ x: 0, y: 0 });
  const scene = view.scene;
  const resources = view.resources;
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
  const layoutCache = useRef<{
    geometry: string;
    width: number;
    height: number;
    layout: ReturnType<typeof fitFrameBoard>;
  } | null>(null);
  const geometry = useMemo(
    () => JSON.stringify(frames.map(({ id, width, height }) => [id, width, height])),
    [frames],
  );
  const boardLayout = useCallback((size: { width: number; height: number }) => {
    const cached = layoutCache.current;
    if (
      cached?.geometry === geometry &&
      cached.width === size.width &&
      cached.height === size.height
    )
      return cached.layout;
    const layout = fitFrameBoard(frames, size);
    layoutCache.current = { geometry, ...size, layout };
    return layout;
  }, [frames, geometry]);
  const content = useCallback(
    (size: { width: number; height: number }) => view.frames
      ? frames.length ? boardLayout(size) : undefined
      : scene?.provenance.viewport,
    [view.frames, frames, boardLayout, scene],
  );
  const {
    host,
    size,
    viewport,
    transform,
    zoom,
    fit,
    moveTo,
    handleKeyDown,
    STEP,
  } = useCanvasTransform(content, view.cameraKey, view.frames ? 0.0001 : 0.05);
  const placements = useMemo(
    () => view.frames
      ? boardLayout(size).frames
      : frames.map((frame) => ({ ...frame, x: 0, y: 0 })),
    [view.frames, boardLayout, size, frames],
  );
  const handleDrag = useCallback((event: Konva.KonvaEventObject<DragEvent>) => {
    if (event.target === stage.current)
      moveTo(event.target.x(), event.target.y());
  }, [moveTo]);
  const findNode = (x: number, y: number) =>
    boardHitTest(
      frames,
      placements,
      (x - transform.x) / transform.scale,
      (y - transform.y) / transform.scale,
      view.requestedRevision,
    );
  const pixelRatio = window.devicePixelRatio || 1;
  // Whole device pixels, so panning does not shift the painted scene by subpixel amounts.
  const snap = (value: number) => Math.round(value * pixelRatio) / pixelRatio;
  const isBackground = (target: EventTarget, current: EventTarget) =>
    target === current ||
    ((!scene || !resources) &&
      target instanceof Element &&
      !target.closest("button, a, input, textarea, [role=button]"));

  return (
    <main
      className="canvas-pane"
      aria-label="Design canvas"
      aria-busy={view.status === "pending"}
    >
      <div
        className="canvas-host"
        ref={host}
        tabIndex={0}
        aria-label="Canvas. Drag to pan. Plus and minus to zoom. F to fit. Select layers in the layers panel."
        onKeyDown={handleKeyDown}
        onPointerDown={(event) => {
          if (
            isBackground(event.target, event.currentTarget) &&
            event.button === 0
          )
            pointerStart.current = { x: event.clientX, y: event.clientY };
        }}
        onPointerUp={(event) => {
          if (
            !isBackground(event.target, event.currentTarget) ||
            event.button !== 0
          )
            return;
          if (
            Math.hypot(
              event.clientX - pointerStart.current.x,
              event.clientY - pointerStart.current.y,
            ) > CLICK_SLOP
          )
            return;
          select("");
          if (document.activeElement instanceof HTMLElement)
            document.activeElement.blur();
        }}
      >
        {frames.length > 0 && (
          <Stage
            ref={stage}
            width={size.width}
            height={size.height}
            x={snap(transform.x)}
            y={snap(transform.y)}
            scaleX={transform.scale}
            scaleY={transform.scale}
            draggable
            onDragMove={handleDrag}
            onDragEnd={handleDrag}
            onPointerDown={() => {
              pointerStart.current = stage.current?.getPointerPosition() ?? {
                x: 0,
                y: 0,
              };
            }}
            onPointerUp={(event) => {
              if (event.evt.button !== 0) return;
              const point = stage.current?.getPointerPosition();
              if (
                !point ||
                Math.hypot(
                  point.x - pointerStart.current.x,
                  point.y - pointerStart.current.y,
                ) > CLICK_SLOP
              )
                return;
              const found = findNode(point.x, point.y);
              select(found?.id ?? "");
              if (!found && document.activeElement instanceof HTMLElement)
                document.activeElement.blur();
            }}
            onContextMenu={(event) => {
              const point = stage.current?.getPointerPosition();
              if (!point) return;
              const found = findNode(point.x, point.y);
              if (found && actions) {
                event.evt.preventDefault();
                actions.open(found.id, event.evt.clientX, event.evt.clientY);
              }
            }}
            onWheel={(event) => {
              event.evt.preventDefault();
              const factor = wheelZoomFactor(event.evt);
              if (factor !== 1)
                zoom(factor, stage.current?.getPointerPosition() ?? undefined);
            }}
          >
            <Layer ref={layer} imageSmoothingEnabled>
              <CanvasScenes
                frames={frames}
                placements={placements}
                scale={transform.scale}
                selectedId={selectedId}
                labels={Boolean(view.frames)}
                board={view.frames ? board : undefined}
              />
            </Layer>
          </Stage>
        )}
        <CanvasMessage view={view} retry={retry} />
      </div>
      <CanvasChrome
        frameName={frameName}
        viewport={
          view.frames && viewport
            ? {
                width: Math.round(viewport.width),
                height: Math.round(viewport.height),
              }
            : viewport
        }
        scale={transform.scale}
        enabled={frames.length > 0}
        zoom={zoom}
        fit={fit}
        step={STEP}
      />
    </main>
  );
}
