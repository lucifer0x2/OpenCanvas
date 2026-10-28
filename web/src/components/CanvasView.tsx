import { useRef } from "react";
import { Stage, Layer, Image as KonvaImage, Rect } from "react-konva";
import type Konva from "konva";
import { hitTest } from "../canvasHitTest";
import { CanvasChrome } from "./CanvasChrome";
import { CanvasMessage } from "./CanvasMessage";
import {
  useCanvasTransform,
  wheelZoomFactor,
} from "../hooks/useCanvasTransform";
import { useDisplayRaster } from "../hooks/useDisplayRaster";
import type { SceneView } from "../hooks/useScene";

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
  const pointerStart = useRef({ x: 0, y: 0 });
  const scene = view.scene;
  const selected = scene?.nodes.find((node) => node.id === selectedId);
  const viewport = scene?.provenance.viewport;
  const { host, size, transform, zoom, fit, moveTo, handleKeyDown, STEP } =
    useCanvasTransform(viewport, scene?.provenance.frameId);
  const pixelRatio = window.devicePixelRatio || 1;
  const bitmap = useDisplayRaster(view, transform.scale, pixelRatio);
  // Whole device pixels, so a raster drawn at the display scale is not resampled by the offset.
  const snap = (value: number) => Math.round(value * pixelRatio) / pixelRatio;

  return (
    <main className="canvas-pane" aria-label="Design canvas">
      <div
        className="canvas-host"
        ref={host}
        tabIndex={0}
        aria-label="Canvas. Drag to pan. Plus and minus to zoom. F to fit. Select layers in the layers panel."
        onKeyDown={handleKeyDown}
      >
        {scene && bitmap && (
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
            onPointerUp={() => {
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
              if (found) select(found.id);
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
              <KonvaImage
                image={bitmap}
                width={viewport!.width}
                height={viewport!.height}
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
