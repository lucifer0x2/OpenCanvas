import { useEffect, useState } from "react";
import { drawScene, loadResources } from "../../../renderer/index";
import type { Resources, Scene } from "../../../renderer/types";
import { fetchSceneJob } from "../sceneJob";
import type { SceneRequest } from "../types";
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
});

export function useScene(
  request: SceneRequest | null,
  retryKey: number,
  preset: string,
) {
  const key = request ? JSON.stringify(request) : "";
  const scope = request
    ? JSON.stringify([request.projectId, request.frameId, preset])
    : "";
  const [loaded, setLoaded] = useState({
    key: "",
    scope: "",
    retryKey,
    view: emptyView(),
  });
  useEffect(() => {
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
      const job = await fetchSceneJob(desired, controller.signal);
      if (stopped) return;
      const scene = job.scene!;
      const resources = await loadResources(scene.fonts, scene.assets, {
        strict: false,
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
