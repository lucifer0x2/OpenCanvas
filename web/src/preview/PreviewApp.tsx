import { useCallback, useEffect, useMemo, useState } from "react";
import { nodeName } from "../state";
import { nodeMotion, rootFrames } from "./model";
import { PreviewStage } from "./PreviewStage";
import { historyDepth, readRoute, routeUrl, type PreviewRoute } from "./route";
import { tokenTable } from "./tokens";
import type { LinkDef } from "./types";
import { usePreviewScenes, usePreviewSnapshot } from "./usePreviewData";
import { useReducedMotion, useToast } from "./usePreviewUi";
import "./preview.css";

/** `/preview`: plays a frame's declared animations, interaction states and links. */
export function PreviewApp() {
  const [route, setRoute] = useState<PreviewRoute>(() =>
    readRoute(location.search),
  );
  const [reload, setReload] = useState(0);
  const [replayKey, setReplayKey] = useState(0);
  const [disabled, setDisabled] = useState(false);
  const [warnings, setWarnings] = useState<string[]>([]);
  const [reducedMotion, setReducedMotion] = useReducedMotion();
  const { toast, show } = useToast();
  const { snapshot, error } = usePreviewSnapshot(route.projectId, reload);
  const frames = useMemo(
    () => (snapshot ? rootFrames(snapshot.nodes) : []),
    [snapshot],
  );
  const frameId = route.frameId || frames[0]?.id || "";
  const view = usePreviewScenes(snapshot, { ...route, frameId });
  const motion = useMemo(() => nodeMotion(snapshot?.nodes ?? []), [snapshot]);
  const tokens = useMemo(
    () => tokenTable(snapshot?.tokens, snapshot?.styles),
    [snapshot],
  );

  useEffect(() => {
    const back = () => setRoute(readRoute(location.search));
    window.addEventListener("popstate", back);
    return () => window.removeEventListener("popstate", back);
  }, []);
  useEffect(() => {
    const frame = frames.find((entry) => entry.id === frameId);
    document.title = frame
      ? `${nodeName(frame)} · Preview`
      : "Preview · OpenCanvas";
  }, [frames, frameId]);

  const go = useCallback(
    (next: string) => {
      const target = { ...route, frameId: next };
      history.pushState(
        { ocPreviewDepth: historyDepth(history.state) + 1 },
        "",
        routeUrl(target),
      );
      setRoute(target);
    },
    [route],
  );
  const onLink = useCallback(
    (link: LinkDef) => {
      const action = link.action ?? "navigate";
      if (action === "navigate" && link.to) {
        if (frames.some((frame) => frame.id === link.to)) go(link.to);
        else show(`Link target ${link.to} is not a root frame.`);
      } else if (action === "back") {
        if (historyDepth(history.state) > 0) history.back();
        else show("Back: there is no previous preview frame.");
      } else show(`"${action}" links are not supported in preview.`);
    },
    [frames, go, show],
  );
  const hasDisabled = !!view.states.disabled;
  const notes = [
    ...(error ? [error] : []),
    ...(view.error ? [view.error] : []),
    ...view.diagnostics,
    ...warnings,
  ];

  return (
    <div className="pv-shell">
      <header className="pv-bar">
        <a className="pv-brand" href="/">
          OpenCanvas
        </a>
        <span className="pv-title">Motion preview</span>
        <label>
          <span>Frame</span>
          <select
            aria-label="Frame"
            value={frameId}
            disabled={!frames.length}
            onChange={(e) => go(e.target.value)}
          >
            {frames.map((frame) => (
              <option key={frame.id} value={frame.id}>
                {nodeName(frame)}
              </option>
            ))}
          </select>
        </label>
        <div className="pv-actions">
          <button
            onClick={() => setReplayKey((key) => key + 1)}
            disabled={view.status !== "ready"}
          >
            Replay
          </button>
          <label className="pv-toggle">
            <input
              type="checkbox"
              checked={reducedMotion}
              onChange={(e) => setReducedMotion(e.target.checked)}
            />
            Reduced motion
          </label>
          {hasDisabled && (
            <label className="pv-toggle">
              <input
                type="checkbox"
                checked={disabled}
                onChange={(e) => setDisabled(e.target.checked)}
              />
              Disabled state
            </label>
          )}
          <button onClick={() => setReload((key) => key + 1)}>Reload</button>
        </div>
      </header>
      {view.status === "ready" && view.scene && view.resources ? (
        <PreviewStage
          scene={view.scene}
          states={view.states}
          resources={view.resources}
          motion={motion}
          tokens={tokens}
          reducedMotion={reducedMotion}
          disabled={disabled}
          replayKey={replayKey}
          onLink={onLink}
          onWarnings={setWarnings}
        />
      ) : (
        <div className="pv-stage pv-message" role="status">
          {error || view.error || "Loading preview…"}
        </div>
      )}
      {notes.length > 0 && (
        <ul className="pv-notes" role="status" aria-label="Preview warnings">
          {notes.map((note) => (
            <li key={note}>{note}</li>
          ))}
        </ul>
      )}
      {toast && (
        <div className="pv-toast" role="alert">
          {toast}
        </div>
      )}
    </div>
  );
}
