import { useCallback, useMemo, useState } from "react";
import { CanvasView } from "./components/CanvasView";
import { FollowToggle } from "./components/FollowToggle";
import { PRESETS, ScopeControls } from "./components/ScopeControls";
import { Inspector } from "./components/Inspector";
import { Layers } from "./components/Layers";
import { Topbar } from "./components/Topbar";
import { NodeActionsProvider } from "./components/NodeActionsProvider";
import { selectionNode } from "./deepLink";
import { useProject } from "./hooks/useProject";
import { useScene } from "./hooks/useScene";
import { usePageScenes } from "./hooks/usePageScenes";
import { ALL_FRAMES, frameDimensions, frameForNode } from "./frameScope";
import { nodeName } from "./state";
import { EMPTY_NODES, nodeIndex } from "./nodeIndex";
import type { SceneRequest } from "./types";
import useLocalStorage from "./hooks/ultility/useLocalStorage";
import { WASM_FALLBACK } from "./scene/service";
import { currentScene, shownRevision } from "./scene/staleFrames";

export function App() {
  const project = useProject();
  const { snapshot, selection } = project;
  const [leftOpen, setLeftOpen] = useState(true);
  const [rightOpen, setRightOpen] = useState(() => window.innerWidth >= 1000);
  const [preset, setPreset] = useState("frame");
  const [retryKey, setRetryKey] = useState(0);
  const nodes = snapshot?.nodes ?? EMPTY_NODES;
  const index = nodeIndex(nodes);
  const pages = useMemo(
    () => (index.childrenByParent.get(null) ?? EMPTY_NODES)
      .filter((node) => node.kind === "page"),
    [index],
  );
  const [isShowNoticeBar, setShowNoticeBar] = useLocalStorage(
    "is_show_notice_bar",
    false,
  );

  const frames = useMemo(
    () => (index.childrenByParent.get(selection.pageId) ?? EMPTY_NODES)
      .filter((node) => node.kind === "frame"),
    [index, selection.pageId],
  );
  const frameIds = useMemo(() => frames.map((frame) => frame.id), [frames]);
  const frame = index.byId.get(selection.frameId);
  const all = selection.frameId === ALL_FRAMES;
  const selectedFrame = useMemo(
    () => frameForNode(nodes, selection.nodeId, selection.pageId),
    [nodes, selection.nodeId, selection.pageId],
  );
  const selectedNode = useMemo(
    () => selectionNode(nodes, selection.nodeId),
    [nodes, selection.nodeId],
  );
  const viewport = PRESETS.find((entry) => entry.key === preset)!;
  const request: SceneRequest | null =
    snapshot && frame
      ? {
          projectId: snapshot.projectId,
          revision: snapshot.revision,
          frameId: frame.id,
          width: viewport.width || frameDimensions(frame).width,
          height: viewport.height || frameDimensions(frame).height,
        }
      : null;
  const singleView = useScene(request, retryKey, preset, snapshot);
  const pageView = usePageScenes(
    all ? snapshot : null,
    selection.pageId,
    frames,
    preset,
    viewport,
    retryKey,
  );
  const view = all ? pageView : singleView;
  // Computed geometry only from a scene that answers exactly the current snapshot.
  const inspected = all
    ? currentScene(
        view.frames?.find((entry) => entry.id === selectedFrame?.id),
        snapshot?.revision,
      )
    : shownRevision(view) === snapshot?.revision
      ? view.scene
      : undefined;
  const inspectorScene =
    inspected?.provenance.projectId === snapshot?.projectId ? inspected : undefined;
  const fallbackNotice = view.diagnostics.find((message) =>
    message.startsWith(WASM_FALLBACK),
  );
  const { selectScope } = project;
  const selectNode = useCallback(
    (nodeId: string) => selectScope({ nodeId }), [selectScope],
  );
  const resolveSelection = useCallback(
    (id: string) => ({
      ...selection,
      nodeId: id,
      frameId: frameForNode(nodes, id, selection.pageId)?.id ?? selection.frameId,
    }), [nodes, selection],
  );
  const sceneStatus =
    view.status === "pending"
      ? view.progress
        ? `${view.progress.ready}/${view.progress.total} frames ready`
        : "Scene pending"
      : view.status === "failed"
        ? "Render failed"
        : view.status === "ready"
          ? `Scene r${shownRevision(view)}`
          : "No scene";

  return (
    <NodeActionsProvider
      selection={selection}
      revision={snapshot?.revision}
      select={selectNode}
      resolveSelection={resolveSelection}
    >
      <div
        className={`app-shell ${leftOpen ? "left-open" : ""} ${rightOpen ? "right-open" : ""}`}
      >
        <Topbar
          projects={project.projects}
          projectId={selection.projectId}
          activity={project.activity}
          loading={project.loadingProjects}
          failed={!!project.error}
          selectProject={project.selectProject}
          revision={
            snapshot?.projectId === selection.projectId
              ? snapshot.revision
              : undefined
          }
          refresh={project.retry}
          frameId={frame?.id ?? selectedFrame?.id ?? ""}
          leftOpen={leftOpen}
          rightOpen={rightOpen}
          toggleLeft={() => setLeftOpen(!leftOpen)}
          toggleRight={() => setRightOpen(!rightOpen)}
          isShowNoticeBar={isShowNoticeBar}
          setShowNoticeBar={setShowNoticeBar}
        />
        <ScopeControls
          pages={pages}
          frames={frames}
          selection={selection}
          select={project.selectScope}
          preset={preset}
          setPreset={setPreset}
        />
        <div className="workspace">
          {leftOpen && (
            <aside className="left-panel">
              <Layers
                nodes={nodes}
                frameId={selection.frameId}
                frameIds={all ? frameIds : undefined}
                selectedId={selection.nodeId}
                select={selectNode}
              />
            </aside>
          )}
          <CanvasView
            view={view}
            selectedId={selection.nodeId}
            select={selectNode}
            retry={() => setRetryKey((key) => key + 1)}
            frameName={
              all
                ? `All frames · ${frames.length}`
                : frame
                  ? nodeName(frame)
                  : ""
            }
          />
          {rightOpen && (
            <Inspector
              node={selectedNode}
              scene={inspectorScene}
              revision={snapshot?.revision}
            />
          )}
        </div>
        {(project.error ||
          project.notice ||
          view.missing.length > 0 ||
          view.diagnostics.length > 0) &&
        (isShowNoticeBar || fallbackNotice) ? (
          <div
            className={`notice-bar ${project.error || view.missing.length ? "warning" : ""}`}
            role="status"
          >
            <span>
              {project.error ||
                fallbackNotice ||
                view.missing.join(" · ") ||
                view.diagnostics.join(" · ") ||
                project.notice}
            </span>
            {project.error && (
              <button onClick={project.retry}>Retry connection</button>
            )}
          </div>
        ) : null}
        <footer className="statusbar">
          <span className={`connection ${project.connection}`}>
            <span className="status-dot" />
            {selection.projectId
              ? project.connection === "live"
                ? "Live"
                : project.connection === "reconnecting"
                  ? "Reconnecting…"
                  : "Connecting…"
              : "Local viewer"}
          </span>
          <FollowToggle follow={project.follow} change={project.setFollow} />
          <div>
            <span>
              {snapshot
                ? `Authored r${snapshot.revision}`
                : project.error
                  ? "Snapshot unavailable"
                  : selection.projectId
                    ? "Loading snapshot…"
                    : "No project selected"}
            </span>
            <span className={view.status === "failed" ? "error-text" : ""}>
              {sceneStatus}
            </span>
          </div>
        </footer>
      </div>
    </NodeActionsProvider>
  );
}
