import type { FontBytesProvider } from "../../../renderer/resource-bytes";
import type { RenderJob, SceneRequest, Snapshot } from "../types";
import { sceneFontBytes } from "./fontBytes";
import { decorateResources, snapshotResources } from "./manifests";
import {
  sceneCacheKey,
  sceneDigest,
  validateBrowserScene,
} from "./sceneIdentity";
import { sceneStore, SCENE_CACHE_ENTRY_BYTES } from "./sceneStore";

export interface BrowserSceneResult {
  job: RenderJob;
  fontBytes?: FontBytesProvider;
  /** Called only after resource readiness and the consumer's cancellation check. */
  persist?: () => void;
}

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
): Promise<BrowserSceneResult> {
  // Never let disk stand in for an authoritative, accepted snapshot.
  const manifests = snapshotResources(snapshot, request);
  const key = sceneCacheKey(request);
  if (key && bypass) void sceneStore.remove(key, signal);
  if (key && !bypass) {
    try {
      const record = await sceneStore.get(key, signal);
      signal.throwIfAborted();
      if (record) {
        if (
          record.key !== key ||
          record.projectId !== request.projectId ||
          record.revision !== request.revision ||
          typeof record.payload !== "string" ||
          !Number.isFinite(record.bytes) ||
          record.bytes <= 0 ||
          record.bytes > SCENE_CACHE_ENTRY_BYTES ||
          record.payload.length > SCENE_CACHE_ENTRY_BYTES ||
          (await sceneDigest(record.payload)) !== record.digest
        )
          throw Error("Invalid cached scene");
        const scene = decorateResources(
          validateBrowserScene(JSON.parse(record.payload), request),
          manifests,
        );
        signal.throwIfAborted();
        return {
          job: {
            ...request,
            jobId: `browser-${scene.provenance.sourceSceneKey}`,
            status: "ready",
            sceneKey: scene.provenance.sourceSceneKey,
            scene,
          },
          fontBytes: sceneFontBytes.get,
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
  const scene = decorateResources(
    validateBrowserScene(result.job.scene!, request),
    manifests,
  );
  return {
    ...result,
    job: { ...result.job, scene },
    persist: () => {
      if (!key || signal.aborted) return;
      // Neither JSON encoding nor a disk write blocks the resource-ready publication.
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
    },
  };
}
