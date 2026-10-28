import { useEffect, useState } from "react";
import { drawScene, loadResources, MAX_PIXELS } from "../../../renderer/index";
import type { Scene } from "../../../renderer/types";
import { projectPath, read } from "../api";
import { acceptsJob } from "../state";
import type { RenderJob, SceneRequest } from "../types";

export interface Raster {
  canvas: HTMLCanvasElement;
  /** The device scale actually rasterized, after the pixel budget clamp. */
  scale: number;
}

export interface SceneView {
  status: "empty" | "pending" | "ready" | "failed";
  scene?: Scene;
  /** The 1:1 rasterization, identical to the worker PNG and the base for cheap previews. */
  canvas?: HTMLCanvasElement;
  /** Rasterizes the same scene at a display scale, so text is drawn at the size it is shown. */
  renderAt?: (scale: number) => Raster | null;
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
    let timer: ReturnType<typeof setTimeout>;
    const controller = new AbortController();
    const started = Date.now();
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

    const receive = async (
      job: RenderJob,
      expectedJobId?: string,
    ): Promise<void> => {
      if (stopped) return;
      if (!acceptsJob(job, desired, expectedJobId)) {
        fail(
          "The render response does not match this revision or viewport. Retry this view.",
        );
        return;
      }
      if (job.status === "failed") {
        fail(
          job.error?.message ?? "The render worker failed. Retry this view.",
        );
        return;
      }
      if (job.status === "pending") {
        if (Date.now() - started > 45_000) {
          fail("Rendering timed out. Retry this view.");
          return;
        }
        timer = setTimeout(() => {
          read<RenderJob>(
            `/api/v1/jobs/${encodeURIComponent(job.jobId)}`,
            controller.signal,
          )
            .then((next) => receive(next, job.jobId))
            .catch((cause: Error) => {
              if (!stopped) fail(cause.message);
            });
        }, 500);
        return;
      }
      const scene = job.scene!;
      const resources = await loadResources(scene.fonts, scene.assets, {
        strict: false,
      });
      if (stopped) return;
      const canvas = document.createElement("canvas");
      drawScene(canvas, scene, resources, { strict: false });
      const { width, height } = scene.provenance.viewport;
      const limit = Math.min(4, Math.sqrt(MAX_PIXELS / (width * height)));
      const renderAt = (scale: number): Raster | null => {
        const wanted = Math.max(0.05, Math.min(limit, scale));
        if (wanted === 1) return { canvas, scale: 1 };
        const target = document.createElement("canvas");
        try {
          drawScene(target, scene, resources, { strict: false, scale: wanted });
        } catch {
          return null;
        }
        return { canvas: target, scale: wanted };
      };
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
        renderAt,
        missing,
        diagnostics: [
          ...new Set([
            ...scene.diagnostics,
            ...(job.diagnostics ?? []),
            ...resources.diagnostics,
          ]),
        ],
      });
    };
    const query = new URLSearchParams({
      frameId: desired.frameId,
      revision: String(desired.revision),
      width: String(desired.width),
      height: String(desired.height),
      theme: "default",
      state: "default",
    });
    read<RenderJob>(
      `${projectPath(desired.projectId)}/scene?${query}`,
      controller.signal,
    )
      .then((job) => receive(job))
      .catch((cause: Error) => {
        if (!stopped) fail(cause.message);
      });
    return () => {
      stopped = true;
      controller.abort();
      clearTimeout(timer);
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
