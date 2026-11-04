import { useCallback, useEffect, useRef, useState } from "react";
import { read } from "../api";
import { bumpRevision, mergeProjects } from "../state";
import type { ActivityEvent, ProjectRef } from "../types";

interface ProjectPage {
  projects: ProjectRef[];
  nextCursor?: string | null;
}

/**
 * The project list. `GET /api/v1/projects` is the source of truth: it loads on mount and retry
 * and is refetched on stream (re)open and visibility changes. The response defines which
 * projects are visible. Live activity can preserve a newer revision of a listed project.
 */
export function useProjectList(reload: number) {
  const [projects, setProjects] = useState<ProjectRef[]>([]);
  const [loaded, setLoaded] = useState(false);
  const [error, setError] = useState("");
  const pending = useRef<AbortController | null>(null);

  const refetch = useCallback(async () => {
    pending.current?.abort();
    const controller = new AbortController();
    pending.current = controller;
    try {
      const incoming: ProjectRef[] = [];
      let cursor: string | null | undefined;
      do {
        const query = cursor ? `?cursor=${encodeURIComponent(cursor)}` : "";
        const page = await read<ProjectPage>(
          `/api/v1/projects${query}`,
          controller.signal,
        );
        if (controller.signal.aborted) return;
        incoming.push(...page.projects);
        cursor = page.nextCursor;
      } while (cursor);
      const visible = new Set(incoming.map(({ projectId }) => projectId));
      setProjects((previous) =>
        mergeProjects(
          previous.filter(({ projectId }) => visible.has(projectId)),
          incoming,
        ),
      );
      setLoaded(true);
      setError("");
    } catch (cause) {
      if (!controller.signal.aborted)
        setError(
          cause instanceof Error ? cause.message : "Projects unavailable.",
        );
    }
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
