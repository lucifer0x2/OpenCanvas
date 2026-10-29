import { useRef } from "react";
import { Stage, Layer, Shape, Rect } from "react-konva";
import type Konva from "konva";
import { paintScene } from "../../../renderer/index";
import { hitTest } from "../canvasHitTest";
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
  const actions = useNodeActions();
  const pointerStart = useRef({ x: 0, y: 0 });
  const scene = view.scene;
  const resources = view.resources;
  const selected = scene?.nodes.find((node) => node.id === selectedId);
  const viewport = scene?.provenance.viewport;
  const { host, size, transform, zoom, fit, moveTo, handleKeyDown, STEP } =
    useCanvasTransform(viewport, scene?.provenance.frameId);
  const pixelRatio = window.devicePixelRatio || 1;
  // Whole device pixels, so panning does not shift the painted scene by subpixel amounts.
  const snap = (value: number) => Math.round(value * pixelRatio) / pixelRatio;
  const isBackground = (target: EventTarget, current: EventTarget) =>
    target === current ||
    ((!scene || !resources) &&
      target instanceof Element &&
      !target.closest("button, a, input, textarea, [role=button]"));

  return (
    <main className="canvas-pane" aria-label="Design canvas">
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
        {scene && resources && (
          <Stage
            ref={stage}
            width={size.width}
            height={size.height}
            x={snap(transform.x)}
            y={snap(transform.y)}
            scaleX={transform.scale}
            scaleY={transform.scale}
            draggable
            onDragEnd={(event) => {
              if (event.target === stage.current)
                moveTo(event.target.x(), event.target.y());
            }}
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
              const found = hitTest(
                scene,
                (point.x - transform.x) / transform.scale,
                (point.y - transform.y) / transform.scale,
              );
              select(found?.id ?? "");
              if (!found && document.activeElement instanceof HTMLElement)
                document.activeElement.blur();
            }}
            onContextMenu={(event) => {
              const point = stage.current?.getPointerPosition();
              if (!point) return;
              const found = hitTest(
                scene,
                (point.x - transform.x) / transform.scale,
                (point.y - transform.y) / transform.scale,
              );
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
            <Layer imageSmoothingEnabled>
              <Rect
                width={viewport!.width}
                height={viewport!.height}
                fill="white"
                shadowColor="#1d2433"
                shadowBlur={20}
                shadowOffsetY={5}
                shadowOpacity={0.12}
              />
              {/* Vectors under the stage transform each draw: sharp at every zoom, no raster. */}
              <Shape
                listening={false}
                sceneFunc={(context) => {
                  context._context.imageSmoothingQuality = "high";
                  paintScene(context._context, scene, resources);
                }}
              />
              {selected && (
                <Rect
                  {...selected.bounds}
                  stroke="#1672e8"
                  strokeWidth={1.5 / transform.scale}
                  listening={false}
                />
              )}
            </Layer>
          </Stage>
        )}
        <CanvasMessage view={view} retry={retry} />
      </div>
      <CanvasChrome
        frameName={frameName}
        viewport={viewport}
        scale={transform.scale}
        enabled={Boolean(scene)}
        zoom={zoom}
        fit={fit}
        step={STEP}
      />
    </main>
  );
}
