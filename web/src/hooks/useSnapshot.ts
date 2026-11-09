import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError } from "../api";
import { readSnapshot } from "../snapshotHttp";
import type { AcceptedSnapshot } from "../snapshotHttp";
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
  const current = useRef<AcceptedSnapshot | null>(null);
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
        const next = await readSnapshot(
          projectId,
          current.current,
          controller.signal,
        );
        if (stopped || !next) return;
        current.current = next;
        setSnapshot(next.snapshot);
        setError("");
        accepted.current(next.snapshot);
      } catch (cause) {
        if (stopped) return;
        if (cause instanceof ApiError && cause.status === 404) {
          current.current = null;
          setSnapshot(null);
        }
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
      current.current?.snapshot.projectId === projectId
        ? current.current.snapshot.revision
        : null,
    [projectId],
  );
  return {
    snapshot: snapshot?.projectId === projectId ? snapshot : null,
    error,
    refresh,
    revision,
  };
}
