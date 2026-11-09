import { useEffect, useMemo, useRef, useState } from "react";
import type { DesignNode, Snapshot } from "../types";
import { frameDimensions } from "../frameScope";
import { diagnosticLabel } from "../diagnostics";
import { nodeName } from "../state";
import { wasmSceneEnabled } from "../scene/flag";
import { loadPageScenes } from "../scene/pageScenes";
import { pageResourceLoader } from "../scene/pageResources";
import { sceneWorker } from "../scene/workerClient";
import type { FrameSceneView, SceneView } from "./useScene";

const empty = (): SceneView => ({
  status: "empty",
  missing: [],
  diagnostics: [],
});

export function usePageScenes(
  snapshot: Snapshot | null,
  pageId: string,
  frames: DesignNode[],
  preset: string,
  viewport: { width: number; height: number },
  retryKey: number,
) {
  const wasm = wasmSceneEnabled();
  const scope = snapshot
    ? JSON.stringify([snapshot.projectId, pageId, "all", preset])
    : "";
  const key = snapshot ? JSON.stringify([scope, snapshot.revision, wasm]) : "";
  // All authored slots exist before an effect or a scene finishes. Their geometry is immutable.
  const targets = useMemo(
    () =>
      frames.map((frame) => ({
        id: frame.id,
        name: nodeName(frame),
        ...(viewport.width ? viewport : frameDimensions(frame)),
      })),
    [key],
  );
  const pending = (): SceneView => ({
    ...empty(),
    status: targets.length ? "pending" : "empty",
    frames: targets,
    progress: { ready: 0, total: targets.length },
  });
  const [loaded, setLoaded] = useState({ key: "", retryKey, view: empty() });
  const lastRetry = useRef(retryKey);
  useEffect(() => {
    const bypassCache = lastRetry.current !== retryKey;
    lastRetry.current = retryKey;
    if (bypassCache && wasm && snapshot) sceneWorker.clearSnapshotCache();
    if (!snapshot) return;
    const controller = new AbortController();
    const { signal } = controller;
    const entries: FrameSceneView[] = [...targets];
    const diagnostics = new Set<string>();
    const missing = new Set<string>();
    const resources = pageResourceLoader();
    const publish = (finished = false, error?: string) => {
      if (signal.aborted) return;
      const ready = entries.find((entry) => entry.scene);
      const failures = entries.filter((entry) => entry.error).length;
      const count = entries.filter((entry) => entry.scene).length;
      setLoaded({
        key,
        retryKey,
        view: {
          status:
            error || (finished && failures)
              ? "failed"
              : !entries.length
                ? "empty"
                : finished
                  ? "ready"
                  : "pending",
          frames: [...entries],
          scene: ready?.scene,
          resources: ready?.resources,
          diagnostics: [...diagnostics],
          missing: [...missing],
          progress: { ready: count, total: entries.length },
          error:
            error ??
            (finished && failures
              ? `${failures} of ${entries.length} frames failed. Retry to render the complete page.`
              : undefined),
        },
      });
    };
    publish();
    const load = async () => {
      await loadPageScenes(
        targets.map((frame) => ({
          projectId: snapshot.projectId,
          revision: snapshot.revision,
          frameId: frame.id,
          width: frame.width,
          height: frame.height,
        })),
        pageId,
        snapshot,
        signal,
        wasm,
        viewport.width ? viewport : undefined,
        {
          bypassCache,
          onFrame: async (result, frameSignal) => {
            frameSignal.throwIfAborted();
            const index = targets.findIndex(
              (frame) => frame.id === result.request.frameId,
            );
            const scene = result.job?.scene;
            if (scene) {
              const loaded = await resources(
                scene,
                frameSignal,
                result.fontBytes,
              );
              frameSignal.throwIfAborted();
              for (const value of [
                ...scene.diagnostics,
                ...(result.job?.diagnostics ?? []),
                ...loaded.diagnostics,
              ])
                diagnostics.add(diagnosticLabel(value));
              for (const item of loaded.readiness)
                if (item.status === "failed")
                  missing.add(`Missing ${item.kind}: ${item.id}`);
              entries[index] = { ...targets[index], scene, resources: loaded };
              publish();
              if (loaded.readiness.every((item) => item.status === "ready"))
                result.persist?.();
            } else {
              entries[index] = {
                ...targets[index],
                error: result.error ?? "Frame unavailable.",
              };
              publish();
            }
          },
        },
      );
      publish(true);
    };
    load().catch((cause) =>
      publish(
        true,
        cause instanceof Error ? cause.message : "Page rendering failed.",
      ),
    );
    return () => controller.abort();
  }, [key, retryKey]);
  const view = !snapshot
    ? empty()
    : loaded.key === key && loaded.retryKey === retryKey
      ? loaded.view
      : pending();
  return { ...view, cameraKey: scope, requestedRevision: snapshot?.revision };
}
