import type { SceneRequest, Snapshot } from "../types.ts";
import { snapshotResources } from "./manifests.ts";
import { sceneWorker } from "./workerClient.ts";

type KeyWorker = Pick<typeof sceneWorker, "keys">;

/**
 * Content digests of frames of one accepted snapshot, in one scene-module pass. A frame
 * missing from the result is computed without a cache. Coded (request-level) errors mean
 * no keys; infrastructure failures propagate, like a failed computation.
 */
export async function browserFrameKeys(
  requests: SceneRequest[],
  snapshot: Snapshot,
  signal: AbortSignal,
  worker: KeyWorker = sceneWorker,
): Promise<Map<string, string>> {
  const keys = new Map<string, string>();
  if (!requests.length) return keys;
  const [first] = requests;
  if (
    requests.some(
      (request) =>
        request.projectId !== first.projectId ||
        request.revision !== first.revision,
    )
  )
    throw new Error("Frame keys require one project revision.");
  const manifests = snapshotResources(snapshot, first);
  let reply;
  try {
    reply = await worker.keys(
      {
        request: first,
        snapshot,
        fonts: [],
        urls: manifests.urls,
        frames: requests.map(({ frameId, width, height }) => ({
          frameId,
          width,
          height,
        })),
      },
      signal,
    );
  } catch (cause) {
    signal.throwIfAborted();
    if (typeof (cause as { code?: unknown })?.code === "string") return keys;
    throw cause;
  }
  if (!Array.isArray(reply) || reply.length !== requests.length) return keys;
  reply.forEach((item, index) => {
    const request = requests[index];
    if (
      item?.frameId === request.frameId &&
      typeof item.digest === "string" &&
      /^[0-9a-f]{64}$/.test(item.digest)
    )
      keys.set(request.frameId, item.digest);
  });
  return keys;
}
