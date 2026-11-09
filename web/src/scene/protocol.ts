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

/** A revision-free digest of one frame's effective render input, or that frame's own error. */
export interface FrameKey {
  frameId: string;
  digest?: string;
  error?: { code: string; message: string };
}

export interface ComputeMessage {
  id: number;
  kind: "prepare" | "compute" | "keys";
  request: SceneRequest;
  /** Frames of `request`'s project and revision, for `keys` only. */
  frames?: { frameId: string; width: number; height: number }[];
  /** Supplied once per cached revision; subsequent requests reference request identity. */
  snapshot?: Snapshot;
  fonts: WorkerFont[];
  urls: ResourceURLs;
}

export type ComputeReply =
  | { id: number; started: "initializing" | "computing" }
  | {
      id: number;
      result: Scene | FontManifest[] | FrameKey[];
      registeredFonts?: { id: string; hash: string }[];
      error?: never;
    }
  | { id: number; error: string; errorCode?: string; result?: never };
