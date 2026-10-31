import { useState } from "react";
import { CanvasView } from "./components/CanvasView";
import { CustomSelect } from "./components/CustomSelect";
import { FollowToggle } from "./components/FollowToggle";
import { Icon } from "./components/Icon";
import { Inspector } from "./components/Inspector";
import { Layers } from "./components/Layers";
import { Topbar } from "./components/Topbar";
import { NodeActionsProvider } from "./components/NodeActionsProvider";
import { selectionNode } from "./deepLink";
import { useProject } from "./hooks/useProject";
import { useScene } from "./hooks/useScene";
import { children, nodeName } from "./state";
import type { SceneRequest } from "./types";

const PRESETS = [
  { key: "frame", name: "Frame size", width: 0, height: 0 },
  { key: "desktop", name: "Desktop", width: 1440, height: 900 },
  { key: "tablet", name: "Tablet", width: 768, height: 1024 },
  { key: "mobile", name: "Mobile", width: 390, height: 844 },
];

export function App() {
  const project = useProject();
  const { snapshot, selection } = project;
  const [leftOpen, setLeftOpen] = useState(true);
  const [rightOpen, setRightOpen] = useState(() => window.innerWidth >= 1000);
  const [preset, setPreset] = useState("frame");
  const [retryKey, setRetryKey] = useState(0);
  const nodes = snapshot?.nodes ?? [];
  const pages = children(nodes, null).filter((node) => node.kind === "page");
  const frames = children(nodes, selection.pageId).filter(
    (node) => node.kind === "frame",
  );
  const frame = nodes.find((node) => node.id === selection.frameId);
  const selectedNode = selectionNode(nodes, selection.nodeId);
  const viewport = PRESETS.find((entry) => entry.key === preset)!;
  const dimension = (value: unknown, fallback: number) =>
    typeof value === "number" && value > 0 ? value : fallback;
  const request: SceneRequest | null =
    snapshot && frame
      ? {
          projectId: snapshot.projectId,
          revision: snapshot.revision,
          frameId: frame.id,
          width: viewport.width || dimension(frame.props.width, 800),
          height: viewport.height || dimension(frame.props.height, 600),
        }
      : null;
  const view = useScene(request, retryKey);
  const selectNode = (nodeId: string) => project.selectScope({ nodeId });
  const sceneStatus =
    view.status === "pending"
      ? "Scene pending"
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
          frameId={frame?.id ?? ""}
          leftOpen={leftOpen}
          rightOpen={rightOpen}
          toggleLeft={() => setLeftOpen(!leftOpen)}
          toggleRight={() => setRightOpen(!rightOpen)}
        />
        <div className="viewbar">
          <div className="scope-controls">
            <label>
              <span>Page</span>
              <CustomSelect
                label="Page"
                disabled={!pages.length}
                value={selection.pageId}
                placeholder="No pages"
                options={pages.map((page) => ({
                  value: page.id,
                  label: nodeName(page),
                }))}
                onChange={(pageId) =>
                  project.selectScope({
                    pageId,
                    frameId: "",
                    nodeId: "",
                  })
                }
              />
            </label>
            <span className="scope-slash">/</span>
            <label>
              <span>Frame</span>
              <CustomSelect
                label="Frame"
                disabled={!frames.length}
                value={selection.frameId}
                placeholder="No frames"
                options={frames.map((entry) => ({
                  value: entry.id,
                  label: nodeName(entry),
                }))}
                onChange={(frameId) =>
                  project.selectScope({
                    frameId,
                    nodeId: frameId,
                  })
                }
              />
            </label>
          </div>
          <label className="viewport-picker">
            <Icon name="frame" />
            <span className="sr-only">Viewport</span>
            <CustomSelect
              label="Viewport"
              value={preset}
              disabled={!frame}
              onChange={setPreset}
              options={PRESETS.map((entry) => ({
                value: entry.key,
                label: entry.name,
                description: entry.width
                  ? `${entry.width} × ${entry.height}`
                  : undefined,
              }))}
            />
          </label>
        </div>
        <div className="workspace">
          {leftOpen && (
            <aside className="left-panel">
              <Layers
                nodes={nodes}
                frameId={selection.frameId}
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
            frameName={frame ? nodeName(frame) : ""}
          />
          {rightOpen && <Inspector node={selectedNode} scene={view.scene} />}
        </div>
        {(project.error ||
          project.notice ||
          view.missing.length > 0 ||
          view.diagnostics.length > 0) && (
          <div
            className={`notice-bar ${project.error || view.missing.length ? "warning" : ""}`}
            role="status"
          >
            <span>
              {project.error ||
                view.missing.join(" · ") ||
                view.diagnostics.join(" · ") ||
                project.notice}
            </span>
            {project.error && (
              <button onClick={project.retry}>Retry connection</button>
            )}
          </div>
        )}
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
