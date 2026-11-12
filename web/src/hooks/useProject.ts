import { useCallback, useEffect, useReducer } from "react";
import { projectSelection } from "../projectSelection";
import { EMPTY_SELECTION, readSelection, saveSelection } from "../state";
import type { ProjectRef, Selection, Snapshot } from "../types";

/** Persisted navigation and its fallback explanation; no fetching or subscriptions. */
export function useProject(projects: ProjectRef[], loaded: boolean) {
  const [{ selection, notice }, dispatch] = useReducer(
    projectSelection,
    undefined,
    () => ({ selection: readSelection(), notice: "" }),
  );
  useEffect(() => {
    saveSelection(selection);
  }, [selection]);
  useEffect(() => {
    if (loaded) dispatch({ type: "projects", projects });
  }, [loaded, projects]);
  const visible =
    !loaded ||
    projects.some((project) => project.projectId === selection.projectId);
  const selectProject = useCallback(
    (projectId: string) => dispatch({ type: "project", projectId }),
    [],
  );
  const accepted = useCallback(
    (snapshot: Snapshot) => dispatch({ type: "accepted", snapshot }),
    [],
  );
  const selectScope = useCallback(
    (snapshot: Snapshot | null, patch: Partial<Selection>) =>
      dispatch({ type: "scope", snapshot, patch }),
    [],
  );
  const setNotice = useCallback(
    (notice: string) => dispatch({ type: "notice", notice }),
    [],
  );
  return {
    selection: visible ? selection : EMPTY_SELECTION,
    notice,
    selectProject,
    selectScope,
    accepted,
    setNotice,
  };
}
