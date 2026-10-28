import { useCallback, useEffect, useRef, useState } from "react";
import { projectPath, read } from "../api";
import type { Snapshot } from "../types";

/**
 * The authoritative HTTP snapshot of one project. Concurrent refreshes coalesce without losing
 * a later one, and an accepted snapshot never moves the revision backwards.
 */
export function useSnapshot(
  projectId: string,
  reload: number,
  onAccepted: (next: Snapshot) => void,
) {
  const [snapshot, setSnapshot] = useState<Snapshot | null>(null);
  const [error, setError] = useState("");
  const current = useRef<Snapshot | null>(null);
  const refreshRef = useRef<() => void>(() => {});
  const accepted = useRef(onAccepted);
  useEffect(() => {
    accepted.current = onAccepted;
  });

  useEffect(() => {
    current.current = null;
    setSnapshot(null);
    setError("");
    refreshRef.current = () => {};
    if (!projectId) return;
    let stopped = false;
    let inFlight = false;
    let dirty = false;
    const controller = new AbortController();
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
        if (!current.current || next.revision >= current.current.revision) {
          current.current = next;
          setSnapshot(next);
          setError("");
          accepted.current(next);
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
    refreshRef.current = () => void refresh();
    void refresh();
    return () => {
      stopped = true;
      controller.abort();
    };
  }, [projectId, reload]);

  const refresh = useCallback(() => refreshRef.current(), []);
  /** Revision of the accepted snapshot of `projectId`, or null before the first one. */
  const revision = useCallback(
    () =>
      current.current?.projectId === projectId
        ? current.current.revision
        : null,
    [projectId],
  );
  return { snapshot, error, refresh, revision };
}
