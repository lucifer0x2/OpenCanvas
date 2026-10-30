import { Icon } from "./Icon";
import { PreviewButton } from "./PreviewButton";
import { ProjectPicker } from "./ProjectPicker";
import { FontManager } from "../fonts/FontManager";
import type { ProjectRef } from "../types";

interface Props {
  projects: ProjectRef[];
  projectId: string;
  activity: string[];
  loading: boolean;
  failed: boolean;
  selectProject: (id: string) => void;
  revision?: number;
  refresh: () => void;
  frameId: string;
  leftOpen: boolean;
  rightOpen: boolean;
  toggleLeft: () => void;
  toggleRight: () => void;
}

/** Project context and viewer/resource actions; scene selection remains in App. */
export function Topbar(props: Props) {
  return (
    <header className="topbar">
      <a href="/" className="brand" aria-label="OpenCanvas home">
        <span className="brand-mark">
          <Icon name="frame" size={19} />
        </span>
        <span>OpenCanvas</span>
      </a>
      <span className="topbar-divider" />
      <ProjectPicker
        projects={props.projects}
        selected={props.projectId}
        activity={props.activity}
        loading={props.loading}
        failed={props.failed}
        select={props.selectProject}
      />
      <span className="read-only-badge">Read only canvas</span>
      <div className="topbar-actions">
        <FontManager
          key={props.projectId}
          projectId={props.projectId}
          projectName={
            props.projects.find((entry) => entry.projectId === props.projectId)
              ?.name ?? "Selected project"
          }
          revision={props.revision}
          changed={props.refresh}
        />
        <PreviewButton
          projectId={props.revision !== undefined ? props.projectId : ""}
          frameId={props.frameId}
        />
        <button
          aria-label={
            props.leftOpen ? "Hide layers panel" : "Show layers panel"
          }
          aria-pressed={props.leftOpen}
          onClick={props.toggleLeft}
        >
          <Icon name="panelLeft" />
        </button>
        <button
          aria-label={
            props.rightOpen ? "Hide inspector panel" : "Show inspector panel"
          }
          aria-pressed={props.rightOpen}
          onClick={props.toggleRight}
        >
          <Icon name="panelRight" />
        </button>
      </div>
    </header>
  );
}
