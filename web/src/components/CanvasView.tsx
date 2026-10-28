import { useEffect, useRef, useState } from "react";
import { Stage, Layer, Image as KonvaImage, Rect } from "react-konva";
import type Konva from "konva";
import { hitTest } from "../canvasHitTest";
import { CanvasMessage } from "./CanvasMessage";
import type { SceneView } from "../hooks/useScene";
import { Icon } from "./Icon";

const clamp = (value: number) => Math.max(0.05, Math.min(4, value));

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
  const host = useRef<HTMLDivElement>(null);
  const stage = useRef<Konva.Stage>(null);
  const [size, setSize] = useState({ width: 600, height: 500 });
  const [transform, setTransform] = useState({ x: 60, y: 60, scale: 1 });
  const pointerStart = useRef({ x: 0, y: 0 });
  const scene = view.scene;
  const selected = scene?.nodes.find((node) => node.id === selectedId);
  const viewport = scene?.provenance.viewport;

  useEffect(() => {
    const observer = new ResizeObserver(([entry]) =>
      setSize({
        width: entry.contentRect.width,
        height: entry.contentRect.height,
      }),
    );
    if (host.current) observer.observe(host.current);
    return () => observer.disconnect();
  }, []);
  const fit = () => {
    if (!viewport) return;
    const scale = clamp(
      Math.min(
        (size.width - 96) / viewport.width,
        (size.height - 120) / viewport.height,
        1,
      ),
    );
    setTransform({
      x: (size.width - viewport.width * scale) / 2,
      y: (size.height - viewport.height * scale) / 2,
      scale,
    });
  };
  useEffect(fit, [
    scene?.provenance.frameId,
    viewport?.width,
    viewport?.height,
    size.width,
    size.height,
  ]);
  const zoom = (
    factor: number,
    point = { x: size.width / 2, y: size.height / 2 },
  ) => {
    setTransform((old) => {
      const scale = clamp(old.scale * factor);
      return {
        scale,
        x: point.x - ((point.x - old.x) / old.scale) * scale,
        y: point.y - ((point.y - old.y) / old.scale) * scale,
      };
    });
  };

  return (
    <main className="canvas-pane" aria-label="Design canvas">
      <div className="canvas-heading">
        <span>{frameName || "Canvas"}</span>
        {viewport && (
          <span>
            {viewport.width} × {viewport.height}
          </span>
        )}
      </div>
      <div
        className="canvas-host"
        ref={host}
        tabIndex={0}
        aria-label="Canvas. Drag to pan. Plus and minus to zoom. F to fit. Select layers in the layers panel."
        onKeyDown={(event) => {
          if (
            [
              "+",
              "=",
              "-",
              "f",
              "F",
              "ArrowUp",
              "ArrowDown",
              "ArrowLeft",
              "ArrowRight",
            ].includes(event.key)
          )
            event.preventDefault();
          if (event.key === "+" || event.key === "=") zoom(1.2);
          if (event.key === "-") zoom(1 / 1.2);
          if (event.key.toLowerCase() === "f") fit();
          const delta = {
            ArrowUp: [0, 40],
            ArrowDown: [0, -40],
            ArrowLeft: [40, 0],
            ArrowRight: [-40, 0],
          }[event.key];
          if (delta)
            setTransform((old) => ({
              ...old,
              x: old.x + delta[0],
              y: old.y + delta[1],
            }));
        }}
      >
        {scene && view.canvas && (
          <Stage
            ref={stage}
            width={size.width}
            height={size.height}
            x={transform.x}
            y={transform.y}
            scaleX={transform.scale}
            scaleY={transform.scale}
            draggable
            onDragEnd={(event) => {
              if (event.target === stage.current)
                setTransform((old) => ({
                  ...old,
                  x: event.target.x(),
                  y: event.target.y(),
                }));
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
                ) > 5
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
              const point = stage.current?.getPointerPosition() ?? undefined;
              zoom(event.evt.deltaY < 0 ? 1.1 : 1 / 1.1, point);
            }}
          >
            <Layer>
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
                image={view.canvas}
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
      <div className="canvas-bottom">
        <span>Drag to pan · scroll to zoom</span>
        <div className="zoom-controls">
          <button
            aria-label="Zoom out"
            disabled={!scene}
            onClick={() => zoom(1 / 1.2)}
          >
            <Icon name="minus" />
          </button>
          <output aria-label="Zoom level">
            {Math.round(transform.scale * 100)}%
          </output>
          <button
            aria-label="Zoom in"
            disabled={!scene}
            onClick={() => zoom(1.2)}
          >
            <Icon name="plus" />
          </button>
          <span className="control-divider" />
          <button
            aria-label="Fit frame"
            title="Fit frame (F)"
            disabled={!scene}
            onClick={fit}
          >
            <Icon name="fit" />
          </button>
        </div>
      </div>
    </main>
  );
}
