import { useCallback, useEffect, useRef, useState } from "react";
import { ApiError, projectPath, read } from "../api";
import { useProjectStream } from "../hooks/useProjectStream";
import { useViewerAccess } from "../hooks/useViewerAccess";
import type { PreviewSnapshot } from "./types";

interface LoadedProject {
  projectId: string;
  snapshot: PreviewSnapshot;
}

/** Public previews watch workspace hints so even a currently hidden link can reopen. */
export function usePreviewProject(projectId: string, reload: number) {
  const readOnly = useViewerAccess();
  const [loaded, setLoaded] = useState<LoadedProject | null>(null);
  const [error, setError] = useState("");
  const current = useRef<LoadedProject | null>(null);
  const pending = useRef<AbortController | null>(null);
  const refresh = useCallback(async () => {
    if (!projectId || readOnly === null) return;
    pending.current?.abort();
    const controller = new AbortController();
    pending.current = controller;
    const { signal } = controller;
    const path = projectPath(projectId);
    try {
      if (readOnly) await read(path, signal);
      // Hints also arrive for other projects. Keep the current preview playing when allowed.
      if (signal.aborted || current.current?.projectId === projectId) return;
      const snapshot = await read<PreviewSnapshot>(`${path}/snapshot`, signal);
      if (signal.aborted) return;
      if (snapshot.truncated) throw new Error("This snapshot is incomplete.");
      current.current = { projectId, snapshot };
      setLoaded(current.current);
      setError("");
    } catch (cause) {
      if (signal.aborted) return;
      if (cause instanceof ApiError && cause.status === 404) {
        current.current = null;
        setLoaded(null);
        setError("Project unavailable.");
      } else if (!current.current) {
        setError(
          cause instanceof Error ? cause.message : "Preview unavailable.",
        );
      }
    }
  }, [projectId, readOnly]);

  useEffect(() => {
    current.current = null;
    setLoaded(null);
    setError(projectId ? "" : "Add ?project=<projectId> to the URL.");
    void refresh();
    return () => pending.current?.abort();
  }, [projectId, refresh, reload]);

  useProjectStream(
    "",
    reload,
    {
      cursor: () => 0,
      onOpen: refresh,
      onReset: refresh,
      onVisibilityChanged: refresh,
      onReconnect: refresh,
      onCommit: () => {},
      onCreated: () => {},
      onActivity: () => {},
    },
    readOnly === true && !!projectId,
  );

  return {
    snapshot: loaded?.projectId === projectId ? loaded.snapshot : null,
    error,
  };
}
