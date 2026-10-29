import { projectPath, read } from "./api";
import { acceptsJob } from "./state";
import type { RenderJob, SceneRequest } from "./types";

export const JOB_POLL_MS = 500;
export const JOB_TIMEOUT_MS = 45_000;

export type SceneState = "default" | "hover" | "pressed" | "focus" | "disabled";

const wait = (ms: number, signal: AbortSignal) =>
  new Promise<void>((resolve, reject) => {
    const timer = setTimeout(resolve, ms);
    signal.addEventListener(
      "abort",
      () => {
        clearTimeout(timer);
        reject(signal.reason);
      },
      { once: true },
    );
  });

/**
 * Requests the computed scene of one frame at the default theme and the given interaction
 * state, polling a pending job every 500 ms for at most 45 s. Resolves with the ready job;
 * rejects with a user-facing message on mismatch, failure or timeout, or when aborted.
 */
export async function fetchSceneJob(
  request: SceneRequest,
  signal: AbortSignal,
  state: SceneState = "default",
): Promise<RenderJob> {
  const started = Date.now();
  const query = new URLSearchParams({
    frameId: request.frameId,
    revision: String(request.revision),
    width: String(request.width),
    height: String(request.height),
    theme: "default",
    state,
  });
  let job = await read<RenderJob>(
    `${projectPath(request.projectId)}/scene?${query}`,
    signal,
  );
  let expectedJobId: string | undefined;
  for (;;) {
    if (!acceptsJob(job, request, expectedJobId, state))
      throw new Error(
        "The render response does not match this revision or viewport. Retry this view.",
      );
    if (job.status === "failed")
      throw new Error(
        job.error?.message ?? "The render worker failed. Retry this view.",
      );
    if (job.status === "ready") return job;
    if (Date.now() - started > JOB_TIMEOUT_MS)
      throw new Error("Rendering timed out. Retry this view.");
    await wait(JOB_POLL_MS, signal);
    expectedJobId = job.jobId;
    job = await read<RenderJob>(
      `/api/v1/jobs/${encodeURIComponent(job.jobId)}`,
      signal,
    );
  }
}
