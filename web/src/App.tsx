import { useState } from "react";
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
import { children, nodeName } from "./state";
import type { SceneRequest } from "./types";
import useLocalStorage from "./hooks/ultility/useLocalStorage";
import { WASM_FALLBACK } from "./scene/service";

export function App() {
  const project = useProject();
  const { snapshot, selection } = project;
  const [leftOpen, setLeftOpen] = useState(true);
  const [rightOpen, setRightOpen] = useState(() => window.innerWidth >= 1000);
  const [preset, setPreset] = useState("frame");
  const [retryKey, setRetryKey] = useState(0);
  const nodes = snapshot?.nodes ?? [];
  const pages = children(nodes, null).filter((node) => node.kind === "page");
  const [isShowNoticeBar, setShowNoticeBar] = useLocalStorage(
    "is_show_notice_bar",
    false,
  );

  const frames = children(nodes, selection.pageId).filter(
    (node) => node.kind === "frame",
  );
  const frame = nodes.find((node) => node.id === selection.frameId);
  const all = selection.frameId === ALL_FRAMES;
  const selectedFrame = frameForNode(nodes, selection.nodeId, selection.pageId);
  const selectedNode = selectionNode(nodes, selection.nodeId);
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
  const inspectorScene = all
    ? view.frames?.find((entry) => entry.id === selectedFrame?.id)?.scene
    : view.scene;
  const fallbackNotice = view.diagnostics.find((message) =>
    message.startsWith(WASM_FALLBACK),
  );
  const selectNode = (nodeId: string) => project.selectScope({ nodeId });
  const sceneStatus =
    view.status === "pending"
      ? view.progress
        ? `${view.progress.ready}/${view.progress.total} frames ready`
        : "Scene pending"
      : view.status === "failed"
        ? "Render failed"
        : view.status === "ready"
          ? `Scene r${view.scene!.provenance.revision}`
          : "No scene";

  return (
    <NodeActionsProvider
      selection={selection}
      revision={snapshot?.revision}
      select={selectNode}
      resolveSelection={(id) => ({
        ...selection,
        nodeId: id,
        frameId:
          frameForNode(nodes, id, selection.pageId)?.id ?? selection.frameId,
      })}
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
                frameIds={all ? frames.map((frame) => frame.id) : undefined}
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
              scene={
                inspectorScene?.provenance.projectId === snapshot?.projectId &&
                inspectorScene?.provenance.revision === snapshot?.revision
                  ? inspectorScene
                  : undefined
              }
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
