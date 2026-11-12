import { EMPTY_SELECTION, pickProject, reconcileSelection } from "./state.ts";
import type { ProjectRef, Selection, Snapshot } from "./types.ts";

export interface ProjectSelection {
  selection: Selection;
  notice: string;
}

export type ProjectSelectionAction =
  | { type: "notice"; notice: string }
  | { type: "project"; projectId: string }
  | { type: "projects"; projects: ProjectRef[] }
  | { type: "accepted"; snapshot: Snapshot }
  | { type: "scope"; snapshot: Snapshot | null; patch: Partial<Selection> };

/** Selection fallback and its explanation change together; all transitions stay pure. */
export function projectSelection(
  state: ProjectSelection,
  action: ProjectSelectionAction,
): ProjectSelection {
  let { selection, notice } = state;
  switch (action.type) {
    case "notice":
      notice = action.notice;
      break;
    case "project":
      selection = { ...EMPTY_SELECTION, projectId: action.projectId };
      notice = "";
      break;
    case "projects": {
      const projectId = pickProject(action.projects, selection.projectId);
      if (projectId === selection.projectId) return state;
      if (selection.projectId)
        notice =
          "The linked or selected project is unavailable. Showing the first available project.";
      selection = { ...EMPTY_SELECTION, projectId };
      break;
    }
    case "accepted": {
      if (selection.projectId !== action.snapshot.projectId) return state;
      const valid = reconcileSelection(action.snapshot, selection);
      if (notice.startsWith("Catching up") || notice.startsWith("Refreshing after"))
        notice = "";
      if (selection.frameId && valid.frameId !== selection.frameId)
        notice =
          "The selected frame was removed. Showing all frames on this page.";
      else if (selection.nodeId && valid.nodeId !== selection.nodeId)
        notice = "The selected layer was removed. Showing its frame.";
      selection = valid;
      break;
    }
    case "scope":
      notice = "";
      if (action.snapshot?.projectId === selection.projectId)
        selection = reconcileSelection(action.snapshot, {
          ...selection,
          ...action.patch,
        });
      break;
  }
  const sameSelection = selection === state.selection ||
    (Object.keys(EMPTY_SELECTION) as (keyof Selection)[]).every(
      (key) => selection[key] === state.selection[key],
    );
  if (sameSelection) selection = state.selection;
  return selection === state.selection && notice === state.notice
    ? state
    : { selection, notice };
}
