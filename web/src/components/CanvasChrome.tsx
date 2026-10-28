import { Icon } from "./Icon";

/** Frame label and zoom controls, floating over the canvas so the pane stays edge to edge. */
export function CanvasChrome({
  frameName,
  viewport,
  scale,
  enabled,
  zoom,
  fit,
  step,
}: {
  frameName: string;
  viewport?: { width: number; height: number };
  scale: number;
  enabled: boolean;
  zoom: (factor: number) => void;
  fit: () => void;
  step: number;
}) {
  return (
    <>
      <div className="canvas-overlay canvas-overlay-top">
        <div className="canvas-heading">
          <span>{frameName || "Canvas"}</span>
          {viewport && (
            <span>
              {viewport.width} × {viewport.height}
            </span>
          )}
        </div>
      </div>
      <div className="canvas-overlay canvas-overlay-bottom">
        <span className="canvas-hint">Drag to pan · scroll to zoom</span>
        <div className="zoom-controls">
          <button
            aria-label="Zoom out"
            disabled={!enabled}
            onClick={() => zoom(1 / step)}
          >
            <Icon name="minus" />
          </button>
          <output aria-label="Zoom level">{Math.round(scale * 100)}%</output>
          <button
            aria-label="Zoom in"
            disabled={!enabled}
            onClick={() => zoom(step)}
          >
            <Icon name="plus" />
          </button>
          <span className="control-divider" />
          <button
            aria-label="Fit frame"
            title="Fit frame (F)"
            disabled={!enabled}
            onClick={fit}
          >
            <Icon name="fit" />
          </button>
        </div>
      </div>
    </>
  );
}
