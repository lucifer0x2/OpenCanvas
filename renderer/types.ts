export const SCENE_VERSION = "scene-3";
export const RENDERER_VERSION = "web-4";
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
  /** Ink rectangle relative to this line's origin/baseline, not its containing box. */
  inkBounds?: Bounds;
}
export interface Measurement {
  width: number;
  height: number;
  lines: TextLine[];
  ascent?: number;
  missing?: number[];
  coverageUnavailable?: boolean;
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
  letterSpacing?: number;
  textTransform?: string;
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
  letterSpacing?: number;
  decoration?: { line: string; offset: number; thickness: number };
  fill: Paint;
  assetId: string | null;
  fit: string;
  opacity: number;
  clip: boolean;
  radius: number | Quad;
  shadow?: SceneShadow[];
  crop: { x: number; y: number; width: number; height: number } | null;
  stroke?: SceneStroke;
}
export type Quad = [number, number, number, number];
export type Paint = string | { type: "linear"; angle: number; stops: [number, string][] }
  | { type: "radial"; center: [number, number]; radius: number | [number, number] | null; stops: [number, string][] };
export interface SceneShadow { x: number; y: number; blur: number; spread: number; color: string; inset: boolean }
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
  diagnostics: Array<string | { code: string; [key: string]: unknown }>;
}
export interface ResourceStatus {
  id: string;
  kind: "font" | "asset";
  status: "ready" | "failed";
  code?: string;
}
export interface Resources {
  glyphCoverage?: Map<string, import("./glyph-coverage").GlyphCoverage>;
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
