import type { SceneView } from "../hooks/useScene";
import { Icon } from "./Icon";

export function CanvasMessage({
  view,
  retry,
}: {
  view: SceneView;
  retry: () => void;
}) {
  // Earlier scenes stay on screen while a revision renders: one quiet, delayed line.
  if (view.revalidating && view.status === "pending")
    return (
      <div className="canvas-refresh quiet" role="status" aria-live="polite">
        <p>
          Updating to r{view.requestedRevision}
          {view.progress
            ? ` · ${view.progress.ready}/${view.progress.total} frames current`
            : ""}{" "}
          · Showing scene r
          {view.scene?.provenance.revision ?? view.requestedRevision}
        </p>
      </div>
    );
  if ((view.scene || view.frames?.length) && view.status !== "ready")
    return (
      <div className="canvas-refresh" role="status" aria-live="polite">
        <h2>
          {view.status === "failed"
            ? "Render failed"
            : `Rendering r${view.requestedRevision}`}
        </h2>
        <p>
          {view.progress
            ? `${view.progress.ready}/${view.progress.total} frames ready · `
            : ""}
          {view.frames ? "All frames" : "Showing scene"} r
          {view.scene?.provenance.revision ?? view.requestedRevision} ·{" "}
          {view.status === "failed"
            ? `Requested r${view.requestedRevision}: ${view.error}`
            : "You can keep panning and zooming."}
        </p>
        {view.status === "failed" && (
          <button onClick={retry}>Retry render</button>
        )}
      </div>
    );
  return (
    <>
      {view.status !== "ready" && (
        <div className="canvas-message" role="status">
          {view.status === "pending" ? (
            <>
              <div className="loading-preview" />
              <h2>Rendering this revision</h2>
              <p>The design will appear when its scene is ready.</p>
            </>
          ) : view.status === "failed" ? (
            <>
              <Icon name="frame" size={28} />
              <h2>Render failed</h2>
              <p>{view.error}</p>
              <button onClick={retry}>Retry render</button>
            </>
          ) : (
            <>
              <Icon name="frame" size={30} />
              <h2>
                {view.frames
                  ? "No frames on this page"
                  : "A clear view of your design"}
              </h2>
              <p>
                {view.frames
                  ? "Choose another page or create a frame through your agent."
                  : "Choose a project and frame to begin."}
                <br />
                Create designs through your agent's MCP tools.
              </p>
            </>
          )}
        </div>
      )}
    </>
  );
}
