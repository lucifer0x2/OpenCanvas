import { SCENE_VERSION, RENDERER_VERSION } from "../../../renderer/types";
import type { Scene } from "../../../renderer/types";
import type { SceneRequest } from "../types";
import { acceptsJob } from "../state";

export const SCENE_CACHE_SCHEMA = 1;
const build =
  typeof __SCENE_BUILD_FINGERPRINT__ === "string"
    ? __SCENE_BUILD_FINGERPRINT__
    : "";

/** Engine is explicit: server scenes never read or populate the browser cache. */
export function sceneCacheKey(request: SceneRequest) {
  return build
    ? JSON.stringify([
        SCENE_CACHE_SCHEMA,
        build,
        "wasm",
        SCENE_VERSION,
        RENDERER_VERSION,
        request.projectId,
        request.revision,
        request.frameId,
        request.width,
        request.height,
        "default",
        "default",
      ])
    : "";
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
