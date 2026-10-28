import { useEffect, useState } from "react";
import { drawScene, loadResources } from "../../../renderer/index";
import type { Scene } from "../../../renderer/types";
import { projectPath, read } from "../api";
import { acceptsJob } from "../state";
import type { RenderJob, SceneRequest } from "../types";

export interface SceneView {
  status: "empty" | "pending" | "ready" | "failed";
  scene?: Scene;
  canvas?: HTMLCanvasElement;
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
