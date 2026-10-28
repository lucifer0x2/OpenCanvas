import { useCallback, useEffect, useRef, useState } from "react";
import { projectPath, read } from "../api";
import {
  commitAction,
  EMPTY_SELECTION,
  readSelection,
  reconcileSelection,
  saveSelection,
} from "../state";
import type {
  CommitEvent,
  Connection,
  ProjectRef,
  Selection,
  Snapshot,
} from "../types";

export function useProject() {
  const [projects, setProjects] = useState<ProjectRef[]>([]);
  const [loadingProjects, setLoadingProjects] = useState(true);
  const [selection, setSelection] = useState<Selection>(readSelection);
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [connection, setConnection] = useState<Connection>("connecting");
  const [error, setError] = useState("");
  const [notice, setNotice] = useState("");
  const [reload, setReload] = useState(0);
  const snapshotRef = useRef<Snapshot | null>(null);
  const retry = useCallback(() => setReload((value) => value + 1), []);

  useEffect(() => {
    const controller = new AbortController();
    setLoadingProjects(true);
    read<{ projects: ProjectRef[] }>("/api/v1/projects", controller.signal)
      .then((result) => {
        setProjects(result.projects);
        setError("");
        setSelection((previous) => ({
          ...previous,
          projectId: result.projects.some(
            (project) => project.projectId === previous.projectId,
          )
            ? previous.projectId
            : (result.projects[0]?.projectId ?? ""),
        }));
      })
      .catch((cause: Error) => {
        if (!controller.signal.aborted) setError(cause.message);
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoadingProjects(false);
      });
    return () => controller.abort();
  }, [reload]);

  useEffect(() => {
    saveSelection(selection);
  }, [selection]);

  useEffect(() => {
    const projectId = selection.projectId;
    if (!projectId) return;
    let stopped = false;
    let inFlight = false;
    let dirty = false;
    let stream: EventSource | undefined;
    let timer: ReturnType<typeof setTimeout>;
    let attempts = 0;
    const controller = new AbortController();
    snapshotRef.current = null;
    setSnapshot(null);
    setError("");
    setConnection("connecting");

    const refresh = async () => {
      if (inFlight) {
        dirty = true;
        return;
      }
      inFlight = true;
      try {
        const next = await read<Snapshot>(
          `${projectPath(projectId)}/snapshot`,
          controller.signal,
        );
        if (stopped || next.projectId !== projectId) return;
        if (next.truncated)
          throw new Error(
            "This snapshot is incomplete. Retry or reduce the project size.",
          );
        if (
          !snapshotRef.current ||
          next.revision >= snapshotRef.current.revision
        ) {
          snapshotRef.current = next;
          setSnapshot(next);
          setError("");
          setNotice((previous) =>
            previous.startsWith("Catching up") ||
            previous.startsWith("Refreshing after")
              ? ""
              : previous,
          );
          setSelection((previous) => {
            if (previous.projectId !== projectId) return previous;
            const valid = reconcileSelection(next, previous);
            if (previous.frameId && valid.frameId !== previous.frameId)
              setNotice(
                "The selected frame was removed. Showing the first available frame.",
              );
            else if (previous.nodeId && valid.nodeId !== previous.nodeId)
              setNotice("The selected layer was removed. Showing its frame.");
            return valid;
          });
        }
      } catch (cause) {
        if (!stopped)
          setError(
            cause instanceof Error ? cause.message : "Snapshot unavailable.",
          );
      } finally {
        inFlight = false;
        if (dirty && !stopped) {
          dirty = false;
          void refresh();
        }
      }
    };

    const connect = () => {
      if (stopped) return;
      const revision = snapshotRef.current?.revision ?? 0;
      const cursor = encodeURIComponent(`${projectId}:${revision}`);
      stream = new EventSource(
        `${projectPath(projectId)}/events?cursor=${cursor}`,
      );
      stream.onopen = () => {
        attempts = 0;
        setConnection("live");
        // Reconcile even when the durable cursor has fallen out of retention.
        void refresh();
      };
      stream.addEventListener("commit", (message) => {
        try {
          const event = JSON.parse(
            (message as MessageEvent).data,
          ) as CommitEvent;
          const action = commitAction(
            event,
            projectId,
            snapshotRef.current?.revision ?? -1,
          );
          if (action === "gap") setNotice("Catching up with changes…");
          if (action !== "ignore") void refresh();
        } catch {
          void refresh();
        }
      });
      stream.addEventListener("reset_required", () => {
        setNotice("Refreshing after an event history reset…");
        void refresh();
      });
      stream.onerror = () => {
        stream?.close();
        if (stopped) return;
        setConnection("reconnecting");
        // Reopen with the last HTTP snapshot cursor, rather than an unverified event ID.
        timer = setTimeout(
          connect,
          Math.min(8000, 500 * 2 ** attempts++) + Math.random() * 250,
        );
      };
    };
    connect();
    void refresh();
    return () => {
      stopped = true;
      controller.abort();
      stream?.close();
      clearTimeout(timer);
    };
  }, [selection.projectId, reload]);

  const selectProject = (projectId: string) => {
    setNotice("");
    setSelection({ ...EMPTY_SELECTION, projectId });
  };
  const selectScope = (patch: Partial<Selection>) => {
    setNotice("");
    setSelection((previous) =>
      snapshot
        ? reconcileSelection(snapshot, { ...previous, ...patch })
        : previous,
    );
  };
  return {
    projects,
    loadingProjects,
    selection,
    snapshot,
    connection,
    error,
    notice,
    retry,
    selectProject,
    selectScope,
  };
}
