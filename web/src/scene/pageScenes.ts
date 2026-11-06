import { projectPath, read } from "../api";
import { RenderJobFailed, settleSceneJob } from "../sceneJob";
import type { RenderJob, SceneRequest, Snapshot } from "../types";
import {
  computeBrowserScene,
  loadBrowserScene,
  WASM_FALLBACK,
} from "./service";
import { yieldFrame } from "./yieldFrame";
import { sceneFontBytes } from "./fontBytes";
import { SCENE_VERSION, RENDERER_VERSION } from "../../../renderer/types";

export type FrameResult = {
  request: SceneRequest;
  job?: RenderJob;
  fontBytes?: Awaited<ReturnType<typeof computeBrowserScene>>["fontBytes"];
  error?: string;
  persist?: () => void;
};
interface PageBatch {
  projectId: string;
  revision: number;
  pageId: string;
  jobs: RenderJob[];
  nextOffset: number | null;
  total: number;
}

/** A single worker computes sequentially and reuses its accepted snapshot and font caches. */
export async function loadPageScenes(
  requests: SceneRequest[],
  pageId: string,
  snapshot: Snapshot,
  signal: AbortSignal,
  wasm: boolean,
  override?: { width: number; height: number },
  options: {
    onFrame?: (result: FrameResult, signal: AbortSignal) => Promise<void>;
    bypassCache?: boolean;
  } = {},
): Promise<FrameResult[]> {
  if (!requests.length) return [];
  let fallback = "";
  const results: FrameResult[] = [];
  if (wasm) {
    for (const request of requests) {
      await yieldFrame(signal);
      let result: FrameResult;
      try {
        result = {
          request,
          ...(await loadBrowserScene(
            request,
            snapshot,
            signal,
            options.bypassCache,
          )),
        };
      } catch (cause) {
        signal.throwIfAborted();
        fallback = `${WASM_FALLBACK}: ${cause instanceof Error ? cause.message : "Browser computation failed."}`;
        break;
      }
      await options.onFrame?.(result, signal);
      signal.throwIfAborted();
      results.push(result);
    }
    if (!fallback) return results;
  }
  const first = requests[0];
  // Browser successes are a completed prefix. Never re-admit them on fallback.
  let offset: number | null = results.length;
  while (offset !== null) {
    await yieldFrame(signal);
    const query = new URLSearchParams({
      pageId,
      revision: String(first.revision),
      offset: String(offset),
      limit: "4",
    });
    if (override) {
      query.set("width", String(override.width));
      query.set("height", String(override.height));
    }
    const batch: PageBatch = await read(
      `${projectPath(first.projectId)}/scenes?${query}`,
      signal,
    );
    const expected = requests.slice(offset, offset + 4);
    const next: number | null =
      offset + expected.length < requests.length
        ? offset + expected.length
        : null;
    if (
      batch.projectId !== first.projectId ||
      batch.revision !== first.revision ||
      batch.pageId !== pageId ||
      batch.total !== requests.length ||
      batch.nextOffset !== next ||
      batch.jobs.length !== expected.length ||
      batch.jobs.some((job, i) => job.frameId !== expected[i].frameId)
    )
      throw new Error(
        "The page render response does not match these frames. Retry this view.",
      );
    // Every job, including failed jobs, reaches a terminal state before another batch is queued.
    const polling = new AbortController();
    const abort = () => polling.abort(signal.reason);
    signal.addEventListener("abort", abort, { once: true });
    if (signal.aborted) abort();
    let completed: FrameResult[];
    try {
      completed = await Promise.all(
        expected.map(async (request, i): Promise<FrameResult> => {
          let result: FrameResult;
          try {
            const job = await settleSceneJob(
              batch.jobs[i],
              request,
              polling.signal,
            );
            if (
              job.scene?.provenance.sceneVersion !== SCENE_VERSION ||
              job.scene.provenance.rendererVersion !== RENDERER_VERSION
            )
              throw new Error(
                "Scene pipeline version mismatch. Retry this frame.",
              );
            result = {
              request,
              fontBytes: sceneFontBytes.get,
              job: {
                ...job,
                diagnostics: [
                  ...(job.diagnostics ?? []),
                  ...(fallback ? [fallback] : []),
                ],
              },
            };
          } catch (cause) {
            signal.throwIfAborted();
            if (!(cause instanceof RenderJobFailed)) {
              polling.abort(cause);
              throw cause;
            }
            result = {
              request,
              error:
                cause instanceof Error
                  ? cause.message
                  : "Frame rendering failed.",
            };
          }
          polling.signal.throwIfAborted();
          try {
            await options.onFrame?.(result, polling.signal);
            polling.signal.throwIfAborted();
          } catch (cause) {
            polling.abort(cause);
            throw cause;
          }
          return result;
        }),
      );
    } finally {
      signal.removeEventListener("abort", abort);
    }
    results.push(...completed);
    offset = batch.nextOffset;
  }
  return results;
}
