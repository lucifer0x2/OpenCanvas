import { SCENE_VERSION, RENDERER_VERSION } from "../../../renderer/types.ts";
import { fetchSceneJob } from "../sceneJob.ts";
import { acceptsJob } from "../state.ts";
import type { RenderJob, SceneRequest, Snapshot } from "../types.ts";
import type { FontBytesProvider } from "../../../renderer/resource-bytes.ts";
import { sceneFontBytes, untilAborted } from "./fontBytes.ts";
import {
  decorateFonts,
  decorateResources,
  snapshotResources,
} from "./manifests.ts";
import { sceneWorker } from "./workerClient.ts";

interface SceneResult {
  job: RenderJob;
  fontBytes?: FontBytesProvider;
}

export const WASM_FALLBACK = "Browser scene unavailable; using server scene";

/** No snapshot reads here: SSE recomputation only starts from useSnapshot's accepted state. */
export async function computeBrowserScene(
  request: SceneRequest,
  snapshot: Snapshot,
  signal: AbortSignal,
): Promise<SceneResult> {
  const manifests = snapshotResources(snapshot, request);
  const message = {
    request,
    snapshot,
    fonts: [],
    urls: manifests.urls,
  };
  const required = await sceneWorker.prepare(message, signal);
  const fonts = decorateFonts(required, manifests);
  const bytes = await untilAborted(
    Promise.all(
      fonts.map(async (font) => ({
        id: font.id,
        hash: font.hash,
        bytes: await sceneFontBytes.get(font),
      })),
    ),
    signal,
  );
  const computed = await sceneWorker.compute(
    {
      ...message,
      fonts: bytes,
    },
    signal,
  );
  const scene = decorateResources(computed, manifests);
  const job: RenderJob = {
    jobId: `browser-${scene.provenance.sourceSceneKey}`,
    status: "ready",
    projectId: request.projectId,
    revision: request.revision,
    frameId: request.frameId,
    sceneKey: scene.provenance.sourceSceneKey,
    scene,
  };
  if (
    !acceptsJob(job, request) ||
    !job.sceneKey ||
    scene.schemaVersion !== 1 ||
    scene.coordinateSpace !== "frame-local" ||
    scene.provenance.sceneVersion !== SCENE_VERSION ||
    scene.provenance.rendererVersion !== RENDERER_VERSION
  )
    throw new Error(
      "Browser scene does not match this revision, viewport or pipeline.",
    );
  return {
    job,
    fontBytes: (font, timeout) => {
      const found = bytes.find(
        (item) => item.id === font.id && item.hash === font.hash,
      );
      return found
        ? Promise.resolve(found.bytes)
        : sceneFontBytes.get(font, timeout);
    },
  };
}

/** WASM errors have a visible server fallback; explicit server selection skips browser compute. */
export async function loadScene(
  request: SceneRequest,
  snapshot: Snapshot | null,
  signal: AbortSignal,
  wasm: boolean,
  browser = computeBrowserScene,
  server = fetchSceneJob,
): Promise<SceneResult> {
  if (!wasm) return { job: await server(request, signal) };
  try {
    if (!snapshot) throw new Error("The authored snapshot is unavailable.");
    return await browser(request, snapshot, signal);
  } catch (cause) {
    signal.throwIfAborted();
    const reason =
      cause instanceof Error ? cause.message : "Browser computation failed.";
    const job = await server(request, signal);
    return {
      job: {
        ...job,
        diagnostics: [
          ...(job.diagnostics ?? []),
          `${WASM_FALLBACK}: ${reason}`,
        ],
      },
      fontBytes: sceneFontBytes.get,
    };
  }
}
