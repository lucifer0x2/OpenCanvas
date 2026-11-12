import type { Selection, Snapshot } from "../types";
import type { useProjectLive } from "../hooks/useProjectLive";

/** The information consumed by the rendered workspace and connection status. */
export interface ProjectView extends ReturnType<typeof useProjectLive> {
  snapshot: Snapshot | null;
  selection: Selection;
  selectScope: (patch: Partial<Selection>) => void;
  notice: string;
  error: string;
}
