import { useCallback, useEffect, useMemo, useState } from "react";
import { readNavigation } from "../deepLink";
import {
  projectSelection,
  type ProjectSelectionAction,
} from "../projectSelection";
import { EMPTY_SELECTION } from "../state";
import type { ProjectRef, Selection, Snapshot } from "../types";
import { useNavigationUrl, writeNavigation } from "./useNavigationUrl";

/** URL-owned navigation and address-scoped explanations; no server-data copy. */
export function useProject(projects: ProjectRef[], loaded: boolean) {
  const href = useNavigationUrl();
  const selection = useMemo(() => readNavigation(href), [href]);
  const [message, setMessage] = useState({ href, text: "" });
  const notice = message.href === href ? message.text : "";
  const setNotice = useCallback((text: string) => {
    const href = window.location.href;
    setMessage((previous) =>
      previous.href === href && previous.text === text
        ? previous
        : { href, text },
    );
  }, []);
  const transition = useCallback(
    (action: ProjectSelectionAction, mode: "push" | "replace") => {
      if (window.location.href !== href) return;
      const next = projectSelection({ selection, notice }, action);
      const target = writeNavigation(next.selection, mode, href);
      setMessage((previous) =>
        previous.href === target && previous.text === next.notice
          ? previous
          : { href: target, text: next.notice },
      );
    },
    [href, selection, notice],
  );
  // User-event handlers stay stable for memoized canvas/layer consumers. Read the
  // live address at invocation; only passive reconciliation captures an old href.
  const navigate = useCallback((action: ProjectSelectionAction) => {
    const href = window.location.href;
    const selection = readNavigation(href);
    if (action.type === "project" && action.projectId === selection.projectId)
      return;
    const next = projectSelection({ selection, notice: "" }, action);
    const target = writeNavigation(next.selection, "push", href);
    setMessage((previous) =>
      previous.href === target && previous.text === next.notice
        ? previous
        : { href: target, text: next.notice },
    );
  }, []);
  useEffect(() => {
    if (loaded) transition({ type: "projects", projects }, "replace");
  }, [loaded, projects, transition]);
  const visible =
    !loaded ||
    projects.some((project) => project.projectId === selection.projectId);
  const selectProject = useCallback(
    (projectId: string) => navigate({ type: "project", projectId }),
    [navigate],
  );
  const accepted = useCallback(
    (snapshot: Snapshot) => {
      // Foreign/late project responses never rewrite the current address.
      if (snapshot.projectId !== selection.projectId) return;
      // Reconcile unless the authoritative list omits the project; the
      // `projects` transition replaces it then. A failed or pending list does
      // not block defaults or removed-target explanations.
      if (
        loaded &&
        !projects.some((project) => project.projectId === selection.projectId)
      )
        return;
      transition({ type: "accepted", snapshot }, "replace");
    },
    [loaded, projects, selection.projectId, transition],
  );
  const selectScope = useCallback(
    (snapshot: Snapshot | null, patch: Partial<Selection>) =>
      navigate({ type: "scope", snapshot, patch }),
    [navigate],
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
