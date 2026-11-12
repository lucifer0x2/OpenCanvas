import { useCallback, useEffect, useMemo } from "react";
import useSWR, { useSWRConfig } from "swr";
import { read } from "../api";
import { bumpRevision } from "../state";
import { snapshotResource } from "../data/snapshotResource";
import type { ActivityEvent, ProjectRef } from "../types";

interface ProjectPage {
  projects: ProjectRef[];
  nextCursor?: string | null;
}
const LIST_KEY = ["projects", "visible", "all-pages"] as const;
const HINTS_KEY = ["project-revision-hints"] as const;
const EMPTY: ProjectRef[] = [];
const NO_HINTS: Record<string, number> = {};

async function listProjects() {
  const projects: ProjectRef[] = [];
  let cursor: string | null | undefined;
  const seen = new Set<string>();
  do {
    const page = await read<ProjectPage>(
      `/api/v1/projects${cursor ? `?cursor=${encodeURIComponent(cursor)}` : ""}`,
    );
    projects.push(...page.projects);
    cursor = page.nextCursor;
    if (cursor && seen.has(cursor))
      throw new Error(
        "The project list could not be completed. Retry this view.",
      );
    if (cursor) seen.add(cursor);
  } while (cursor);
  return projects;
}

/** HTTP owns membership; revision hints cannot insert an unlisted project. */
export function useProjectList() {
  const { cache, mutate } = useSWRConfig();
  const query = useSWR<ProjectRef[]>(LIST_KEY, listProjects);
  const hints = useSWR<Record<string, number>>(HINTS_KEY, null);
  useEffect(() => {
    if (query.data)
      snapshotResource(cache, mutate).reconcileVisible(
        query.data.map((project) => project.projectId),
      );
  }, [cache, mutate, query.data]);
  const projects = useMemo(
    () =>
      Object.entries(hints.data ?? NO_HINTS).reduce(
        (entries, [projectId, revision]) =>
          bumpRevision(entries, { projectId, revision }),
        query.data ?? EMPTY,
      ),
    [query.data, hints.data],
  );
  const refetch = useCallback(() => {
    return query.mutate().catch(() => undefined);
  }, [query.mutate]);
  const bump = useCallback(
    (event: ActivityEvent) => {
      if (!Number.isSafeInteger(event.revision)) return;
      void hints.mutate(
        (previous = {}) =>
          previous[event.projectId] >= event.revision
            ? previous
            : { ...previous, [event.projectId]: event.revision },
        { revalidate: false },
      );
    },
    [hints.mutate],
  );
  return {
    projects,
    loaded: query.data !== undefined,
    loading: query.isLoading,
    error: query.error instanceof Error ? query.error.message : "",
    refetch,
    bump,
  };
}
