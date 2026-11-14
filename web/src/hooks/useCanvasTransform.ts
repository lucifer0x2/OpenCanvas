import { useCallback, useEffect, useLayoutEffect, useRef, useState } from "react";
import type { KeyboardEvent } from "react";

export interface Transform {
  x: number;
  y: number;
  scale: number;
}
export interface Point {
  x: number;
  y: number;
}
interface Viewport {
  width: number;
  height: number;
}

const MIN_SCALE = 0.05;
const MAX_SCALE = 4;
const STEP = 1.2;
// Scale exponent per wheel pixel: a mouse notch still steps noticeably, a trackpad glides.
const WHEEL_SPEED = 0.0012;
const PINCH_SPEED = 0.0035;
// Room left around a fitted frame for the floating chrome.
const FIT_MARGIN = { x: 96, y: 120 };
const PAN_STEP: Record<string, [number, number]> = {
  ArrowUp: [0, 40],
  ArrowDown: [0, -40],
  ArrowLeft: [40, 0],
  ArrowRight: [-40, 0],
};

export const clampScale = (value: number, minimum = MIN_SCALE) =>
  Math.max(minimum, Math.min(MAX_SCALE, value));

/** Zoom factor for one wheel or trackpad step, proportional to the delta that step carries. */
export function wheelZoomFactor(wheel: WheelEvent): number {
  // Lines and pages carry far fewer units than pixels; even them out first.
  const unit = wheel.deltaMode === 1 ? 16 : wheel.deltaMode === 2 ? 120 : 1;
  const delta = Math.max(-80, Math.min(80, wheel.deltaY * unit));
  if (!delta) return 1;
  return Math.exp(-delta * (wheel.ctrlKey ? PINCH_SPEED : WHEEL_SPEED));
}

/** Pan and zoom of the canvas viewport, with the host element it is measured against. */
export function useCanvasTransform(
  content: Viewport | undefined | ((size: Viewport) => Viewport | undefined),
  cameraKey: string | undefined,
  minimumScale = MIN_SCALE,
) {
  const host = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 0, height: 0 });
  const viewport = typeof content === "function" ? content(size) : content;
  const fitted = useRef({ cameraKey, done: false });
  const [transform, setTransform] = useState<Transform>({
    x: 60,
    y: 60,
    scale: 1,
  });

  useEffect(() => {
    const observer = new ResizeObserver(([entry]) =>
      setSize((old) =>
        old.width === entry.contentRect.width && old.height === entry.contentRect.height
          ? old
          : { width: entry.contentRect.width, height: entry.contentRect.height },
      ),
    );
    if (host.current) observer.observe(host.current);
    return () => observer.disconnect();
  }, []);

  const viewportWidth = viewport?.width;
  const viewportHeight = viewport?.height;
  const fit = useCallback(() => {
    if (
      viewportWidth === undefined || viewportHeight === undefined ||
      size.width <= 0 || size.height <= 0
    ) return;
    const scale = clampScale(
      Math.min(
        (size.width - FIT_MARGIN.x) / viewportWidth,
        (size.height - FIT_MARGIN.y) / viewportHeight,
        1,
      ),
      minimumScale,
    );
    setTransform({
      x: (size.width - viewportWidth * scale) / 2,
      y: (size.height - viewportHeight * scale) / 2,
      scale,
    });
  }, [viewportWidth, viewportHeight, size.width, size.height, minimumScale]);
  useLayoutEffect(() => {
    if (fitted.current.cameraKey !== cameraKey)
      fitted.current = { cameraKey, done: false };
    if (
      !fitted.current.done && viewportWidth !== undefined && viewportHeight !== undefined &&
      size.width > 0 && size.height > 0
    ) {
      fit();
      fitted.current.done = true;
    }
  }, [cameraKey, viewportWidth, viewportHeight, size.width, size.height, fit]);

  /** Zoom by `factor`, keeping `point` (host coordinates) over the same scene position. */
  const zoom = useCallback(
    (factor: number, point: Point = { x: size.width / 2, y: size.height / 2 }) =>
      setTransform((old) => {
        const scale = clampScale(old.scale * factor, minimumScale);
        return {
          scale,
          x: point.x - ((point.x - old.x) / old.scale) * scale,
          y: point.y - ((point.y - old.y) / old.scale) * scale,
        };
      }),
    [size.width, size.height, minimumScale],
  );

  const moveTo = useCallback(
    (x: number, y: number) =>
      setTransform((old) => old.x === x && old.y === y ? old : { ...old, x, y }),
    [],
  );

  const handleKeyDown = useCallback((event: KeyboardEvent) => {
    const pan = PAN_STEP[event.key];
    const key = event.key.toLowerCase();
    if (pan || ["+", "=", "-", "f"].includes(key)) event.preventDefault();
    if (event.key === "+" || event.key === "=") zoom(STEP);
    if (event.key === "-") zoom(1 / STEP);
    if (key === "f") fit();
    if (pan)
      setTransform((old) => ({ ...old, x: old.x + pan[0], y: old.y + pan[1] }));
  }, [zoom, fit]);

  return {
    host,
    size,
    viewport,
    transform,
    zoom,
    fit,
    moveTo,
    handleKeyDown,
    STEP,
  };
}
