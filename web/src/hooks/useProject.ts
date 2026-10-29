import { useCallback, useEffect, useState } from "react";
import {
  clearActivity,
  commitAction,
  EMPTY_SELECTION,
  followTarget,
  markActivity,
  pickProject,
  readSelection,
  reconcileSelection,
  saveSelection,
} from "../state";
import type { Selection, Snapshot } from "../types";
import { useFollowAgent } from "./useFollowAgent";
import { useProjectList } from "./useProjectList";
import { useProjectStream } from "./useProjectStream";
import { useSnapshot } from "./useSnapshot";

const TRANSIENT = ["Catching up", "Refreshing after"];

/** Project list, selection, snapshot and the tab's one live stream, composed for `App`. */
export function useProject() {
  const [selection, setSelection] = useState<Selection>(readSelection);
  const [notice, setNotice] = useState("");
  const [reload, setReload] = useState(0);
  const [activity, setActivity] = useState<string[]>([]);
  const [follow, setFollow] = useFollowAgent();
  const retry = useCallback(() => setReload((value) => value + 1), []);
  const projectId = selection.projectId;
  const list = useProjectList(reload);

  const accepted = (next: Snapshot) => {
    setNotice((previous) =>
      TRANSIENT.some((prefix) => previous.startsWith(prefix)) ? "" : previous,
    );
    setSelection((previous) => {
      if (previous.projectId !== next.projectId) return previous;
      const valid = reconcileSelection(next, previous);
      if (previous.frameId && valid.frameId !== previous.frameId)
        setNotice(
          "The selected frame was removed. Showing the first available frame.",
        );
      else if (previous.nodeId && valid.nodeId !== previous.nodeId)
        setNotice("The selected layer was removed. Showing its frame.");
      return valid;
    });
  };
  const snap = useSnapshot(projectId, reload, accepted);

  useEffect(() => {
    saveSelection(selection);
  }, [selection]);

  // A missing or deleted selection falls back to the first project, including the first one
  // created while the workspace was empty.
  useEffect(() => {
    if (!list.loaded) return;
    setSelection((previous) => {
      const next = pickProject(list.projects, previous.projectId);
      if (next !== previous.projectId && previous.projectId)
        setNotice(
          "The linked or selected project is unavailable. Showing the first available project.",
        );
      return next === previous.projectId
        ? previous
        : { ...EMPTY_SELECTION, projectId: next };
    });
  }, [list.loaded, list.projects]);

  const selectProject = useCallback((next: string) => {
    setNotice("");
    setActivity((marked) => clearActivity(marked, next));
    setSelection({ ...EMPTY_SELECTION, projectId: next });
  }, []);
  const nameOf = (id: string) =>
    list.projects.find((entry) => entry.projectId === id)?.name ?? id;
  // A hint from another project: follow it, or report it without touching this snapshot.
  const hinted = (id: string, name: string, created: boolean) => {
    const target = followTarget(follow, id, projectId);
    if (target) {
      selectProject(target);
      if (projectId) setNotice(`Following the agent to ${name}.`);
      return;
    }
    if (id === projectId) return;
    if (created)
      return setNotice(`${name} was created. Select it in Project to view it.`);
    setActivity((marked) => markActivity(marked, id, projectId));
    setNotice(`${name} has new changes. Select it in Project to view them.`);
  };

  const connection = useProjectStream(projectId, reload, {
    cursor: () => snap.revision() ?? 0,
    onOpen: () => {
      list.refetch();
      snap.refresh();
    },
    onCommit: (event) => {
      if (!event) return snap.refresh();
      const action = commitAction(event, projectId, snap.revision() ?? -1);
      if (action === "gap") setNotice("Catching up with changes…");
      if (action !== "ignore") snap.refresh();
    },
    onReset: () => {
      if (projectId) setNotice("Refreshing after an event history reset…");
      list.refetch();
      snap.refresh();
    },
    onCreated: (project) => {
      list.merge([project]);
      hinted(project.projectId, project.name, true);
    },
    onActivity: (event) => {
      list.bump(event);
      hinted(event.projectId, nameOf(event.projectId), false);
    },
  });

  const selectScope = (patch: Partial<Selection>) => {
    setNotice("");
    setSelection((previous) =>
      snap.snapshot
        ? reconcileSelection(snap.snapshot, { ...previous, ...patch })
        : previous,
    );
  };
  return {
    projects: list.projects,
    loadingProjects: list.loading,
    selection,
    snapshot: snap.snapshot,
    connection,
    error: list.error || snap.error,
    notice,
    activity,
    follow,
    setFollow,
    retry,
    selectProject,
    selectScope,
  };
}
