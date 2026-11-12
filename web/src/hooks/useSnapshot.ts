import { useCallback, useEffect } from "react";
import useSWR, { useSWRConfig } from "swr";
import type { AcceptedSnapshot } from "../snapshotHttp";
import type { Snapshot } from "../types";
import {
  snapshotEpochKey,
  snapshotHeadKey,
  snapshotKey,
  snapshotReadKey,
  type SnapshotPointer,
} from "../data/snapshotKeys";
import { snapshotResource } from "../data/snapshotResource";

/** Latest authored snapshot at a revision watermark; scene parameters never enter its key. */
export function useSnapshot(
  projectId: string,
  knownRevision: number | null = null,
) {
  const { cache, mutate } = useSWRConfig();
  const resource = snapshotResource(cache, mutate);
  const { data: epoch = 0 } = useSWR<number>(
    projectId ? snapshotEpochKey(projectId) : null,
    null,
  );
  const { data: head } = useSWR<{ revision: number }>(
    projectId ? snapshotHeadKey(projectId) : null,
    null,
  );
  const query = useSWR<SnapshotPointer>(
    projectId ? snapshotReadKey(projectId, knownRevision, epoch) : null,
    resource.fetch,
    {
      revalidateIfStale: false,
      revalidateOnFocus: false,
      revalidateOnReconnect: false,
    },
  );
  const { data: accepted } = useSWR<AcceptedSnapshot>(
    projectId && head ? snapshotKey(projectId, head.revision) : null,
    null,
  );
  const refresh = useCallback(
    (visibilityChanged = false) =>
      resource.invalidate(projectId, visibilityChanged),
    [projectId, resource],
  );
  const revision = useCallback(
    () => resource.current(projectId)?.snapshot.revision ?? null,
    [projectId, resource],
  );
  return {
    snapshot: accepted?.snapshot ?? null,
    error: query.error instanceof Error ? query.error.message : "",
    refresh,
    revision,
    validation: query.data?.validation,
  };
}

/** A 304 is an accepted validation too, even though the snapshot identity stays stable. */
export function useAcceptedSnapshot(
  snapshot: Snapshot | null,
  validation: number | undefined,
  accept: (snapshot: Snapshot) => void,
) {
  useEffect(() => {
    if (snapshot && validation !== undefined) accept(snapshot);
  }, [accept, snapshot, validation]);
}
