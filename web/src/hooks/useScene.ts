import { useEffect, useState } from "react";
import { drawScene, loadResources } from "../../../renderer/index";
import type { Resources, Scene } from "../../../renderer/types";
import { fetchSceneJob } from "../sceneJob";
import { acceptsJob } from "../state";
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
}

export function useScene(request: SceneRequest | null, retryKey: number) {
  const [view, setView] = useState<SceneView>({
    status: "empty",
    missing: [],
    diagnostics: [],
  });
  const key = request ? JSON.stringify(request) : "";
  useEffect(() => {
    if (!request) {
      setView({ status: "empty", missing: [], diagnostics: [] });
      return;
    }
    const desired = request;
    let stopped = false;
    const controller = new AbortController();
    setView({ status: "pending", missing: [], diagnostics: [] });
    const fail = (message: string) => {
      if (!stopped)
        setView({
          status: "failed",
          missing: [],
          diagnostics: [],
          error: message,
        });
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
      setView({
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
      });
    };
    load().catch((cause: Error) => fail(cause.message));
    return () => {
      stopped = true;
      controller.abort();
    };
    // The serialized immutable request prevents object identity from restarting jobs.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key, retryKey]);
  // Hide the previous request's scene before effects run, including the inspector.
  const exact =
    !view.scene ||
    (request &&
      acceptsJob(
        {
          jobId: "view",
          status: "ready",
          projectId: view.scene.provenance.projectId,
          revision: view.scene.provenance.revision,
          frameId: view.scene.provenance.frameId,
          scene: view.scene,
        },
        request,
      ));
  return exact
    ? view
    : ({ status: "pending", missing: [], diagnostics: [] } satisfies SceneView);
}
