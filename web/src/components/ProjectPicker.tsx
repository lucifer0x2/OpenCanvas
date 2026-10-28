import type { ProjectRef } from "../types";

interface Props {
  projects: ProjectRef[];
  selected: string;
  /** Projects with agent changes the user has not opened yet. */
  activity: string[];
  loading: boolean;
  failed: boolean;
  select: (projectId: string) => void;
}

/** Project selector; a dot marks projects an agent changed while another one was in view. */
export function ProjectPicker(props: Props) {
  const { projects, selected, activity } = props;
  const changed = activity.filter((id) =>
    projects.some((entry) => entry.projectId === id),
  ).length;
  return (
    <>
      <label className="project-picker">
        <span className="sr-only">Project</span>
        <select
          aria-label="Project"
          value={selected}
          onChange={(event) => props.select(event.target.value)}
          disabled={!projects.length}
        >
          {!projects.length && (
            <option value="">
              {props.loading
                ? "Loading projects…"
                : props.failed
                  ? "Projects unavailable"
                  : "No projects yet"}
            </option>
          )}
          {projects.map((entry) => (
            <option key={entry.projectId} value={entry.projectId}>
              {activity.includes(entry.projectId)
                ? `● ${entry.name}`
                : entry.name}
            </option>
          ))}
        </select>
      </label>
      {changed > 0 && (
        <span
          className="connection live"
          role="status"
          aria-label={`${changed} other project${changed > 1 ? "s" : ""} changed`}
          title="Other projects have new agent changes"
        >
          <span className="status-dot" />
          {changed}
        </span>
      )}
    </>
  );
}
