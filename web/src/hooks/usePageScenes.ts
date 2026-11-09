import { useEffect, useMemo, useRef, useState } from "react";
import type { DesignNode, Snapshot } from "../types";
import { frameDimensions } from "../frameScope";
import { diagnosticLabel } from "../diagnostics";
import { nodeName } from "../state";
import { wasmSceneEnabled } from "../scene/flag";
import { loadPageScenes } from "../scene/pageScenes";
import { pageResourceLoader } from "../scene/pageResources";
import {
  failFrame,
  pageView,
  reuseFrame,
  seedFrames,
} from "../scene/staleFrames";
import { sceneWorker } from "../scene/workerClient";
import type { FrameSceneView, SceneView } from "./useScene";

const empty = (): SceneView => ({
  status: "empty",
  missing: [],
  diagnostics: [],
});
type Loaded = { scope: string; key: string; retryKey: number; view: SceneView };

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
  const [loaded, setLoaded] = useState<Loaded>({
    scope: "",
    key: "",
    retryKey,
    view: empty(),
  });
  // The latest published view seeds the next revision of the same scope.
  const latest = useRef(loaded);
  latest.current = loaded;
  // One loader per scope: immutable manifests already loaded stay loaded across revisions.
  const loader = useRef<{
    scope: string;
    load: ReturnType<typeof pageResourceLoader>;
  } | null>(null);
  const lastRetry = useRef(retryKey);
  useEffect(() => {
    const bypassCache = lastRetry.current !== retryKey;
    lastRetry.current = retryKey;
    if (bypassCache && wasm && snapshot) sceneWorker.clearSnapshotCache();
    if (!snapshot) return;
    const controller = new AbortController();
    const { signal } = controller;
    const previous = latest.current;
    // Retry and a new scope start clean; a new revision keeps every rendered frame on screen.
    const seeded =
      !bypassCache &&
      previous.scope === scope &&
      previous.view.frames?.some((f) => f.scene);
    const entries: FrameSceneView[] = seeded
      ? seedFrames(targets, previous.view.frames)
      : targets.map((target) => ({ ...target }));
    // Missing resources get a fresh attempt at the next revision, as before.
    if (
      bypassCache ||
      loader.current?.scope !== scope ||
      previous.view.missing.length
    )
      loader.current = { scope, load: pageResourceLoader() };
    const resources = loader.current.load;
    const publish = (finished = false, error?: string) => {
      if (signal.aborted) return;
      setLoaded({
        scope,
        key,
        retryKey,
        view: pageView(entries, { finished, error, seeded: Boolean(seeded) }),
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
          reuse: (request, contentKey) => {
            const index = targets.findIndex(
              (frame) => frame.id === request.frameId,
            );
            const reused = reuseFrame(entries[index], request, contentKey);
            if (!reused) return false;
            entries[index] = reused;
            publish();
            return true;
          },
          onFrame: async (result, frameSignal) => {
            frameSignal.throwIfAborted();
            const index = targets.findIndex(
              (frame) => frame.id === result.request.frameId,
            );
            const scene = result.job?.scene;
            if (!scene) {
              entries[index] = failFrame(
                entries[index],
                targets[index],
                result.error ?? "Frame unavailable.",
              );
              publish();
              return;
            }
            const loaded = await resources(
              scene,
              frameSignal,
              result.fontBytes,
            );
            frameSignal.throwIfAborted();
            entries[index] = {
              ...targets[index],
              scene,
              resources: loaded,
              contentKey: result.contentKey,
              diagnostics: [
                ...scene.diagnostics,
                ...(result.job?.diagnostics ?? []),
                ...loaded.diagnostics,
              ].map(diagnosticLabel),
              missing: loaded.readiness
                .filter((item) => item.status === "failed")
                .map((item) => `Missing ${item.kind}: ${item.id}`),
            };
            publish();
            if (loaded.readiness.every((item) => item.status === "ready"))
              result.persist?.();
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
      : transition(loaded, scope, retryKey, targets);
  return { ...view, cameraKey: scope, requestedRevision: snapshot?.revision };
}

/** Before the effect's first publish: keep the scope's scenes, else show placeholders. */
function transition(
  loaded: Loaded,
  scope: string,
  retryKey: number,
  targets: { id: string; name: string; width: number; height: number }[],
): SceneView {
  const keep =
    loaded.scope === scope &&
    loaded.retryKey === retryKey &&
    loaded.view.frames?.some((frame) => frame.scene);
  return pageView(
    keep
      ? seedFrames(targets, loaded.view.frames)
      : targets.map((t) => ({ ...t })),
    { seeded: Boolean(keep) },
  );
}
