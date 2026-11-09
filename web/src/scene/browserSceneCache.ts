import type { FontBytesProvider } from "../../../renderer/resource-bytes";
import type { Scene } from "../../../renderer/types";
import type { RenderJob, SceneRequest, Snapshot } from "../types";
import { sceneFontBytes } from "./fontBytes";
import { decorateResources, snapshotResources } from "./manifests";
import {
  frameContentKey,
  rebaseScene,
  sceneCacheKey,
  sceneDigest,
  validateBrowserScene,
} from "./sceneIdentity";
import { sceneMemory } from "./sceneMemory";
import { sceneStore, SCENE_CACHE_ENTRY_BYTES } from "./sceneStore";

export interface BrowserSceneResult {
  job: RenderJob;
  fontBytes?: FontBytesProvider;
  /** Called only after resource readiness and the consumer's cancellation check. */
  persist?: () => void;
}

const readyJob = (request: SceneRequest, scene: Scene): RenderJob => ({
  ...request,
  jobId: `browser-${scene.provenance.sourceSceneKey}`,
  status: "ready",
  sceneKey: scene.provenance.sourceSceneKey,
  scene,
});

/**
 * Memory, then disk, then compute, all keyed by the frame's content `digest` (empty: no
 * cache). A hit computed under an older revision is restamped to this accepted request.
 */
export async function cachedBrowserScene(
  request: SceneRequest,
  snapshot: Snapshot,
  signal: AbortSignal,
  compute: (
    request: SceneRequest,
    snapshot: Snapshot,
    signal: AbortSignal,
  ) => Promise<BrowserSceneResult>,
  bypass = false,
  digest = "",
): Promise<BrowserSceneResult> {
  // Never let a cache stand in for an authoritative, accepted snapshot.
  const manifests = snapshotResources(snapshot, request);
  const memoryKey = frameContentKey(request, digest);
  const key = sceneCacheKey(request, digest);
  // Only content-key hits are restamped; computed scenes must already match the request.
  const accept = (scene: Scene, hit = true) =>
    decorateResources(
      validateBrowserScene(hit ? rebaseScene(scene, request) : scene, request),
      manifests,
    );
  const remember = (scene: Scene) => () => {
    if (!signal.aborted) sceneMemory.set(memoryKey, scene);
  };
  if (bypass) {
    sceneMemory.remove(memoryKey);
    if (key) void sceneStore.remove(key, signal);
  } else {
    const held = sceneMemory.get(memoryKey);
    if (held) {
      try {
        const scene = accept(held);
        return { job: readyJob(request, scene), fontBytes: sceneFontBytes.get };
      } catch {
        sceneMemory.remove(memoryKey);
      }
    }
  }
  if (key && !bypass) {
    try {
      const record = await sceneStore.get(key, signal);
      signal.throwIfAborted();
      if (record) {
        if (
          record.key !== key ||
          record.projectId !== request.projectId ||
          typeof record.payload !== "string" ||
          !Number.isFinite(record.bytes) ||
          record.bytes <= 0 ||
          record.bytes > SCENE_CACHE_ENTRY_BYTES ||
          record.payload.length > SCENE_CACHE_ENTRY_BYTES ||
          (await sceneDigest(record.payload)) !== record.digest
        )
          throw Error("Invalid cached scene");
        const scene = accept(JSON.parse(record.payload));
        signal.throwIfAborted();
        return {
          job: readyJob(request, scene),
          fontBytes: sceneFontBytes.get,
          persist: remember(scene),
        };
      }
    } catch {
      signal.throwIfAborted();
      void sceneStore.remove(key, signal);
    }
  }
  signal.throwIfAborted();
  const result = await compute(request, snapshot, signal);
  signal.throwIfAborted();
  const scene = accept(result.job.scene!, false);
  return {
    ...result,
    job: { ...result.job, scene },
    persist: () => {
      remember(scene)();
      if (key) persistScene(key, request, scene, signal);
    },
  };
}

/** Neither JSON encoding nor a disk write blocks the resource-ready publication. */
function persistScene(
  key: string,
  request: SceneRequest,
  scene: Scene,
  signal: AbortSignal,
) {
  if (signal.aborted) return;
  const save = () => {
    void (async () => {
      if (signal.aborted) return;
      const payload = JSON.stringify(scene);
      const bytes = new TextEncoder().encode(payload).byteLength;
      if (bytes > SCENE_CACHE_ENTRY_BYTES) return;
      const digest = await sceneDigest(payload);
      if (signal.aborted) return;
      await sceneStore.put(
        {
          key,
          projectId: request.projectId,
          revision: request.revision,
          payload,
          digest,
          bytes,
          used: Date.now(),
        },
        signal,
      );
    })().catch(() => {
      /* Storage is optional, including denied crypto or quota. */
    });
  };
  if (typeof requestIdleCallback === "function")
    requestIdleCallback(save, { timeout: 1500 });
  else if (typeof requestAnimationFrame === "function")
    requestAnimationFrame(() => setTimeout(save, 0));
  else setTimeout(save, 0);
}
