export const SCENE_VERSION = "scene-2";
export const RENDERER_VERSION = "web-2";
export const MAX_PIXELS = 16_000_000;
export interface FontManifest {
  id: string;
  family: string;
  hash: string;
  url: string;
  weight: number;
  style: string;
}
export interface AssetManifest {
  id: string;
  hash: string;
  url: string;
  mediaType: string;
}
export interface TextLine {
  text: string;
  x: number;
  y: number;
  width: number;
}
export interface Measurement {
  width: number;
  height: number;
  lines: TextLine[];
}
export interface MeasureRequest {
  key: string;
  text: string;
  fontId: string;
  fontHash: string;
  family: string;
  weight: number;
  style: string;
  fontSize: number;
  lineHeight: number;
  width: number | null;
}
export interface Bounds {
  x: number;
  y: number;
  width: number;
  height: number;
}
export interface SceneNode {
  id: string;
  parentId: string | null;
  kind: string;
  bounds: Bounds;
  lines: TextLine[];
  fontId: string | null;
  fontSize: number;
  lineHeight: number;
  fill: string;
  assetId: string | null;
  fit: string;
  opacity: number;
  clip: boolean;
  radius: number;
  crop: { x: number; y: number; width: number; height: number } | null;
  stroke?: SceneStroke;
}
/** Resolved stroke, sides in CSS order [top, right, bottom, left]. */
export interface SceneStroke {
  color: [string, string, string, string];
  width: [number, number, number, number];
  align: "inside" | "center" | "outside";
}
export interface Provenance {
  projectId: string;
  revision: number;
  frameId: string;
  viewport: { width: number; height: number };
  theme: string;
  state: string;
  sceneVersion: string;
  rendererVersion: string;
  sourceSceneKey: string;
  fontManifestHash: string;
  assetManifestHash: string;
}
export interface Scene {
  schemaVersion: number;
  coordinateSpace: string;
  provenance: Provenance;
  nodes: SceneNode[];
  fonts: FontManifest[];
  assets: AssetManifest[];
  diagnostics: string[];
}
export interface ResourceStatus {
  id: string;
  kind: "font" | "asset";
  status: "ready" | "failed";
  code?: string;
}
export interface Resources {
  fonts: Map<string, FontManifest>;
  images: Map<string, HTMLImageElement>;
  assetHashes: Map<string, string>;
  readiness: ResourceStatus[];
  diagnostics: string[];
}
export interface LayoutInput {
  schemaVersion: number;
  projectId: string;
  revision: number;
  frameId: string;
  viewport: { width: number; height: number };
  theme?: string;
  state?: string;
  nodes: {
    id: string;
    parentId: string | null;
    kind: string;
    props: Record<string, unknown>;
  }[];
  fonts: FontManifest[];
  assets: AssetManifest[];
}
