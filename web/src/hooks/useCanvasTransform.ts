import { useEffect, useRef, useState } from "react";
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

export const clampScale = (value: number) =>
  Math.max(MIN_SCALE, Math.min(MAX_SCALE, value));

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
  viewport: Viewport | undefined,
  frameId: string | undefined,
) {
  const host = useRef<HTMLDivElement>(null);
  const [size, setSize] = useState({ width: 600, height: 500 });
  const [transform, setTransform] = useState<Transform>({
    x: 60,
    y: 60,
    scale: 1,
  });

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
    const scale = clampScale(
      Math.min(
        (size.width - FIT_MARGIN.x) / viewport.width,
        (size.height - FIT_MARGIN.y) / viewport.height,
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
    frameId,
    viewport?.width,
    viewport?.height,
    size.width,
    size.height,
  ]);

  /** Zoom by `factor`, keeping `point` (host coordinates) over the same scene position. */
  const zoom = (
    factor: number,
    point: Point = { x: size.width / 2, y: size.height / 2 },
  ) =>
    setTransform((old) => {
      const scale = clampScale(old.scale * factor);
      return {
        scale,
        x: point.x - ((point.x - old.x) / old.scale) * scale,
        y: point.y - ((point.y - old.y) / old.scale) * scale,
      };
    });

  const moveTo = (x: number, y: number) =>
    setTransform((old) => ({ ...old, x, y }));

  const handleKeyDown = (event: KeyboardEvent) => {
    const pan = PAN_STEP[event.key];
    const key = event.key.toLowerCase();
    if (pan || ["+", "=", "-", "f"].includes(key)) event.preventDefault();
    if (event.key === "+" || event.key === "=") zoom(STEP);
    if (event.key === "-") zoom(1 / STEP);
    if (key === "f") fit();
    if (pan)
      setTransform((old) => ({ ...old, x: old.x + pan[0], y: old.y + pan[1] }));
  };

  return { host, size, transform, zoom, fit, moveTo, handleKeyDown, STEP };
}
