import { useCallback, useEffect, useRef, useState } from "react";
import { read } from "../api";
import { bumpRevision, mergeProjects } from "../state";
import type { ActivityEvent, ProjectRef } from "../types";

/**
 * The project list. `GET /api/v1/projects` is the source of truth: it loads on mount and retry
 * and is refetched on every stream (re)open, because the live hints merged in between are not
 * durable. Projects are never removed, so a late response cannot drop a hinted project.
 */
export function useProjectList(reload: number) {
  const [projects, setProjects] = useState<ProjectRef[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef<AbortController | null>(null);

  const refetch = useCallback(() => {
    pending.current?.abort();
    const controller = new AbortController();
    pending.current = controller;
    read<{ projects: ProjectRef[] }>("/api/v1/projects", controller.signal)
      .then((result) => {
        setProjects((previous) => mergeProjects(previous, result.projects));
        setLoaded(true);
        setError("");
      })
      .catch((cause: Error) => {
        if (!controller.signal.aborted) setError(cause.message);
      });
  }, []);

  useEffect(() => {
    refetch();
    return () => pending.current?.abort();
  }, [refetch, reload]);

  const merge = useCallback(
    (incoming: ProjectRef[]) =>
      setProjects((previous) => mergeProjects(previous, incoming)),
    [],
  );
  const bump = useCallback(
    (event: ActivityEvent) =>
      setProjects((previous) => bumpRevision(previous, event)),
    [],
  );
  return {
    projects,
    loaded,
    loading: !loaded && !error,
    error,
    refetch,
    merge,
    bump,
  };
}
