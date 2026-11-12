export const snapshotHeadKey = (project: string) =>
  ["snapshot-head", project] as const;
export const snapshotEpochKey = (project: string) =>
  ["snapshot-epoch", project] as const;
export const snapshotKey = (project: string, revision: number) =>
  ["snapshot", project, revision, "object", "visible"] as const;

/** This endpoint returns latest, so revision is a lower bound, never a historical identity. */
export const snapshotReadKey = (
  project: string,
  revision: number | null,
  epoch: number,
) =>
  ["snapshot-latest", project, revision, epoch, "object", "visible"] as const;
export type SnapshotReadKey = ReturnType<typeof snapshotReadKey>;
export interface SnapshotPointer {
  revision: number | null;
  validation: number;
}
export interface SnapshotHead {
  revision: number;
  epoch: number;
}
