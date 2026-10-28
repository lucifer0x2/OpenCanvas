import { useState } from "react";
import { CanvasView } from "./components/CanvasView";
import { Icon } from "./components/Icon";
import { Inspector } from "./components/Inspector";
import { Layers } from "./components/Layers";
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
  const selectedNode = nodes.find((node) => node.id === selection.nodeId);
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
    <div
      className={`app-shell ${leftOpen ? "left-open" : ""} ${rightOpen ? "right-open" : ""}`}
    >
      <header className="topbar">
        <a href="/" className="brand" aria-label="OpenCanvas home">
          <span className="brand-mark">
            <Icon name="frame" size={19} />
          </span>
          <span>OpenCanvas</span>
        </a>
        <span className="topbar-divider" />
        <label className="project-picker">
          <span className="sr-only">Project</span>
          <select
            aria-label="Project"
            value={selection.projectId}
            onChange={(event) => project.selectProject(event.target.value)}
            disabled={!project.projects.length}
          >
            {!project.projects.length && (
              <option value="">
                {project.loadingProjects
                  ? "Loading projects…"
                  : project.error
                    ? "Projects unavailable"
                    : "No projects yet"}
              </option>
            )}
            {project.projects.map((entry) => (
              <option key={entry.projectId} value={entry.projectId}>
                {entry.name}
              </option>
            ))}
          </select>
        </label>
        <span className="read-only-badge">Read only</span>
        <div className="topbar-actions">
          <button
            aria-label={leftOpen ? "Hide layers panel" : "Show layers panel"}
            aria-pressed={leftOpen}
            onClick={() => setLeftOpen(!leftOpen)}
          >
            <Icon name="panelLeft" />
          </button>
          <button
            aria-label={
              rightOpen ? "Hide inspector panel" : "Show inspector panel"
            }
            aria-pressed={rightOpen}
            onClick={() => setRightOpen(!rightOpen)}
          >
            <Icon name="panelRight" />
          </button>
        </div>
      </header>
      <div className="viewbar">
        <div className="scope-controls">
          <label>
            <span>Page</span>
            <select
              aria-label="Page"
              disabled={!pages.length}
              value={selection.pageId}
              onChange={(event) =>
                project.selectScope({
                  pageId: event.target.value,
                  frameId: "",
                  nodeId: "",
                })
              }
            >
              {!pages.length && <option value="">No pages</option>}
              {pages.map((page) => (
                <option key={page.id} value={page.id}>
                  {nodeName(page)}
                </option>
              ))}
            </select>
          </label>
          <span className="scope-slash">/</span>
          <label>
            <span>Frame</span>
            <select
              aria-label="Frame"
              disabled={!frames.length}
              value={selection.frameId}
              onChange={(event) =>
                project.selectScope({
                  frameId: event.target.value,
                  nodeId: event.target.value,
                })
              }
            >
              {!frames.length && <option value="">No frames</option>}
              {frames.map((entry) => (
                <option key={entry.id} value={entry.id}>
                  {nodeName(entry)}
                </option>
              ))}
            </select>
          </label>
        </div>
        <label className="viewport-picker">
          <Icon name="frame" />
          <span className="sr-only">Viewport</span>
          <select
            aria-label="Viewport"
            value={preset}
            disabled={!frame}
            onChange={(event) => setPreset(event.target.value)}
          >
            {PRESETS.map((entry) => (
              <option key={entry.key} value={entry.key}>
                {entry.name}
                {entry.width ? ` · ${entry.width} × ${entry.height}` : ""}
              </option>
            ))}
          </select>
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
  );
}
