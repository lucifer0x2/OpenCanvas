import type { FontManifest, Scene } from "../../../renderer/types.ts";
import type { SceneRequest, Snapshot } from "../types.ts";

export interface WorkerFont {
  id: string;
  hash: string;
  bytes: ArrayBuffer;
}

export interface ResourceURLs {
  fonts: Record<string, string>;
  assets: Record<string, string>;
}

export interface ComputeMessage {
  id: number;
  kind: "prepare" | "compute";
  request: SceneRequest;
  /** Supplied once per cached revision; subsequent requests reference request identity. */
  snapshot?: Snapshot;
  fonts: WorkerFont[];
  urls: ResourceURLs;
}

export type ComputeReply =
  | { id: number; started: "initializing" | "computing" }
  | {
      id: number;
      result: Scene | FontManifest[];
      registeredFonts?: { id: string; hash: string }[];
      error?: never;
    }
  | { id: number; error: string; errorCode?: string; result?: never };
