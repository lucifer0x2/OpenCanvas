import { useEffect, useMemo, useRef, useState } from "react";
import { drawScene, loadResources } from "../../../renderer/index";
import type { Resources, Scene } from "../../../renderer/types";
import { loadScene } from "../scene/service";
import { wasmSceneEnabled } from "../scene/flag";
import { sceneWorker } from "../scene/workerClient";
import type { SceneRequest, Snapshot } from "../types";
import { diagnosticLabel } from "../diagnostics";

export interface SceneView {
  status: "empty" | "pending" | "ready" | "failed";
  scene?: Scene;
  /** The 1:1 rasterization, identical to the worker PNG: the pipeline check and parity artifact. */
  canvas?: HTMLCanvasElement;
  /** Loaded fonts and images, so the canvas paints the scene as vectors at any zoom. */
  resources?: Resources;
  missing: string[];
  diagnostics: string[];
  error?: string;
  /** Requested project/frame/preset identity; revisions and authored dimensions are excluded. */
  cameraKey?: string;
  requestedRevision?: number;
  frames?: FrameSceneView[];
  progress?: { ready: number; total: number };
  /** Accepted revision `scene` is shown for; a reused board scene keeps an older provenance. */
  sceneRevision?: number;
  /** An earlier revision stays on screen while this one renders; no placeholder is due. */
  revalidating?: boolean;
}

export interface FrameSceneView {
  id: string;
  name: string;
  width: number;
  height: number;
  scene?: Scene;
  resources?: Resources;
  /**
   * Accepted revision `scene` answers. A reused scene keeps its object (and its older
   * `provenance.revision`) so paint caches hold; compare this field, never provenance.
   */
  revision?: number;
  error?: string;
  /** Revision-free identity of `scene`'s render input, when the scene module supplied one. */
  contentKey?: string;
  /** `scene` is from an earlier revision and this one has not replaced it yet. */
  stale?: boolean;
  /** Revalidation failed; `scene` stays as last rendered. */
  updateError?: string;
  diagnostics?: string[];
  missing?: string[];
}

const EMPTY_VIEW: SceneView = {
  status: "empty",
  missing: [],
  diagnostics: [],
};
const EMPTY_REQUEST: SceneRequest = {
  projectId: "", revision: 0, frameId: "", width: 0, height: 0,
};
const refreshing = (view: SceneView): SceneView => ({
  ...view,
  status: "pending",
  error: undefined,
  revalidating: Boolean(view.scene),
});

export function useScene(
  request: SceneRequest | null,
  retryKey: number,
  preset: string,
  snapshot: Snapshot | null = null,
) {
  const wasm = wasmSceneEnabled();
  const enabled = request !== null;
  const { projectId, revision, frameId, width, height } = request ?? EMPTY_REQUEST;
  const desired = useMemo(
    () => enabled ? { projectId, revision, frameId, width, height } : null,
    [enabled, projectId, revision, frameId, width, height],
  );
  const key = desired ? JSON.stringify([desired, wasm]) : "";
  const scope = desired
    ? JSON.stringify([desired.projectId, desired.frameId, preset])
    : "";
  const [loaded, setLoaded] = useState({
    key: "",
    scope: "",
    retryKey,
    view: EMPTY_VIEW,
  });
  const lastRetry = useRef(retryKey);
  useEffect(() => {
    const bypassCache = lastRetry.current !== retryKey;
    if (bypassCache) {
      lastRetry.current = retryKey;
      // Retry resends the accepted snapshot, even when its revision is unchanged.
      if (wasm && desired) sceneWorker.clearSnapshotCache();
    }
    if (!desired) {
      setLoaded((old) => old.view === EMPTY_VIEW
        ? old
        : { key, scope, retryKey, view: EMPTY_VIEW },
      );
      return;
    }
    let stopped = false;
    const controller = new AbortController();
    setLoaded((old) => ({
      key,
      scope,
      retryKey,
      view: refreshing(old.scope === scope ? old.view : EMPTY_VIEW),
    }));
    const fail = (message: string) => {
      if (!stopped)
        setLoaded((old) => ({
          ...old,
          view: { ...old.view, status: "failed", error: message },
        }));
    };
    const load = async () => {
      const { job, fontBytes, persist } = await loadScene(
        desired,
        snapshot,
        controller.signal,
        wasm,
        undefined,
        undefined,
        bypassCache,
      );
      if (stopped) return;
      const scene = job.scene!;
      const resources = await loadResources(scene.fonts, scene.assets, {
        strict: false,
        fontBytes,
      });
      if (stopped) return;
      const canvas = document.createElement("canvas");
      drawScene(canvas, scene, resources, { strict: false });
      const missing = resources.readiness
        .filter((resource) => resource.status === "failed")
        .map(
          (resource) =>
            `${resource.kind === "font" ? "Missing font" : "Missing asset"}: ${resource.id}`,
        );
      setLoaded({
        key,
        scope,
        retryKey,
        view: {
          status: "ready",
          scene,
          canvas,
          resources,
          missing,
          diagnostics: [
            ...new Set(
              [
                ...scene.diagnostics,
                ...(job.diagnostics ?? []),
                ...resources.diagnostics,
              ].map(diagnosticLabel),
            ),
          ],
        },
      });
      if (resources.readiness.every((item) => item.status === "ready"))
        persist?.();
    };
    load().catch((cause: Error) => fail(cause.message));
    return () => {
      stopped = true;
      controller.abort();
    };
  }, [desired, snapshot, wasm, key, scope, retryKey]);
  // Derive before effects: retain only this scope, and never label an older request ready.
  const view = useMemo(
    () => !desired
      ? EMPTY_VIEW
      : loaded.key === key && loaded.scope === scope && loaded.retryKey === retryKey
        ? loaded.view
        : refreshing(loaded.scope === scope ? loaded.view : EMPTY_VIEW),
    [desired, loaded, key, scope, retryKey],
  );
  return useMemo(
    () => ({ ...view, cameraKey: scope, requestedRevision: desired?.revision }),
    [view, scope, desired?.revision],
  );
}
