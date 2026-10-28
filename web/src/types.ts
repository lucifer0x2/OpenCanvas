import type { Scene } from "../../renderer/types";

export interface ProjectRef {
  projectId: string;
  slug: string;
  name: string;
  revision: number;
}

export interface DesignNode {
  id: string;
  parentId: string | null;
  sortKey: number;
  kind: string;
  props: Record<string, unknown>;
}

export interface Snapshot extends ProjectRef {
  schemaVersion: number;
  nodes: DesignNode[];
  truncated: boolean;
}

export interface Selection {
  projectId: string;
  pageId: string;
  frameId: string;
  nodeId: string;
}

export interface Viewport {
  width: number;
  height: number;
}
export interface SceneRequest extends Viewport {
  projectId: string;
  revision: number;
  frameId: string;
}

export interface RenderJob {
  jobId: string;
  status: "pending" | "ready" | "failed";
  projectId: string;
  revision: number;
  frameId: string;
  sceneKey?: string;
  scene?: Scene;
  diagnostics?: string[];
  error?: { code: string; message: string };
}

export type Connection = "connecting" | "live" | "reconnecting";
export interface CommitEvent {
  projectId: string;
  revision: number;
}
