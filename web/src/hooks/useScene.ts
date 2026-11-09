import { useEffect, useRef, useState } from "react";
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

const emptyView = (): SceneView => ({
  status: "empty",
  missing: [],
  diagnostics: [],
});
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
  const key = request ? JSON.stringify([request, wasm]) : "";
  const scope = request
    ? JSON.stringify([request.projectId, request.frameId, preset])
    : "";
  const [loaded, setLoaded] = useState({
    key: "",
    scope: "",
    retryKey,
    view: emptyView(),
  });
  const lastRetry = useRef(retryKey);
  useEffect(() => {
    const bypassCache = lastRetry.current !== retryKey;
    if (bypassCache) {
      lastRetry.current = retryKey;
      // Retry resends the accepted snapshot, even when its revision is unchanged.
      if (wasm && request) sceneWorker.clearSnapshotCache();
    }
    if (!request) {
      setLoaded({ key, scope, retryKey, view: emptyView() });
      return;
    }
    const desired = request;
    let stopped = false;
    const controller = new AbortController();
    setLoaded((old) => ({
      key,
      scope,
      retryKey,
      view: refreshing(old.scope === scope ? old.view : emptyView()),
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
    // The serialized immutable request prevents object identity from restarting jobs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, scope, retryKey]);
  // Derive before effects: retain only this scope, and never label an older request ready.
  const view = !request
    ? emptyView()
    : loaded.key === key &&
        loaded.scope === scope &&
        loaded.retryKey === retryKey
      ? loaded.view
      : refreshing(loaded.scope === scope ? loaded.view : emptyView());
  return { ...view, cameraKey: scope, requestedRevision: request?.revision };
}
