import { SCENE_VERSION, RENDERER_VERSION } from "../../../renderer/types";
import type { Scene } from "../../../renderer/types";
import type { SceneRequest } from "../types";
import { acceptsJob } from "../state";

/** 2: entries are keyed by a frame content digest instead of the project revision. */
export const SCENE_CACHE_SCHEMA = 2;
const build =
  typeof __SCENE_BUILD_FINGERPRINT__ === "string"
    ? __SCENE_BUILD_FINGERPRINT__
    : "";

/**
 * In-memory identity: the frame's revision-free input digest (from the scene module) plus
 * the pipeline and viewport. Equal keys compute equal scenes, apart from provenance revision.
 */
export function frameContentKey(request: SceneRequest, digest: string) {
  return /^[0-9a-f]{64}$/.test(digest)
    ? JSON.stringify([
        SCENE_CACHE_SCHEMA,
        build,
        "wasm",
        SCENE_VERSION,
        RENDERER_VERSION,
        request.projectId,
        digest,
        request.frameId,
        request.width,
        request.height,
        "default",
        "default",
      ])
    : "";
}

/** Engine is explicit: server scenes never read or populate the browser cache. */
export function sceneCacheKey(request: SceneRequest, digest: string) {
  return build ? frameContentKey(request, digest) : "";
}

/** Persisted keys of an older schema or build can never be read again. */
export function currentCacheKey(key: unknown) {
  return (
    typeof key === "string" &&
    key.startsWith(
      JSON.stringify([SCENE_CACHE_SCHEMA, build]).slice(0, -1) + ",",
    )
  );
}

/**
 * Restamp a scene whose content key matched under a newer accepted revision. Only the
 * provenance revision differs; callers still validate the result against `request`.
 */
export function rebaseScene(scene: Scene, request: SceneRequest): Scene {
  return scene.provenance.revision === request.revision
    ? scene
    : {
        ...scene,
        provenance: { ...scene.provenance, revision: request.revision },
      };
}

/** Check the boundary before persisting, after JSON decode and before painting. */
export function validateBrowserScene(scene: Scene, request: SceneRequest) {
  const finite = (value: unknown) =>
    typeof value === "number" && Number.isFinite(value);
  if (
    !scene ||
    scene.schemaVersion !== 1 ||
    scene.coordinateSpace !== "frame-local" ||
    !scene.provenance?.sourceSceneKey ||
    scene.provenance.sceneVersion !== SCENE_VERSION ||
    scene.provenance.rendererVersion !== RENDERER_VERSION ||
    !acceptsJob(
      { ...request, jobId: "cache-check", status: "ready", scene },
      request,
    ) ||
    ![scene.nodes, scene.fonts, scene.assets, scene.diagnostics].every(
      Array.isArray,
    ) ||
    scene.nodes.some(
      (node) =>
        !node ||
        typeof node.id !== "string" ||
        typeof node.kind !== "string" ||
        !node.bounds ||
        ![
          node.bounds.x,
          node.bounds.y,
          node.bounds.width,
          node.bounds.height,
          node.opacity,
          node.fontSize,
          node.lineHeight,
        ].every(finite) ||
        !Array.isArray(node.lines) ||
        node.lines.some(
          (line) =>
            typeof line.text !== "string" ||
            ![line.x, line.y, line.width].every(finite),
        ),
    )
  )
    throw new Error(
      "Browser scene does not match this revision, viewport or pipeline.",
    );
  return scene;
}

export async function sceneDigest(payload: string) {
  const bytes = new TextEncoder().encode(payload);
  const digest = await crypto.subtle.digest("SHA-256", bytes);
  return Array.from(new Uint8Array(digest), (byte) =>
    byte.toString(16).padStart(2, "0"),
  ).join("");
}
