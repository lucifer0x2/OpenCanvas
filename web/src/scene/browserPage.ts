import type { SceneRequest, Snapshot } from "../types";
import { browserFrameKeys } from "./frameKeys";
import type { FrameResult, PageSceneOptions } from "./pageScenes";
import { frameContentKey } from "./sceneIdentity";
import { loadBrowserScene, WASM_FALLBACK } from "./service";
import { yieldFrame } from "./yieldFrame";

/**
 * One key pass, then each frame in order: reused by the consumer, read from cache or
 * computed. Stops at the first browser failure; results stay a completed prefix.
 */
export async function browserPagePass(
  requests: SceneRequest[],
  snapshot: Snapshot,
  signal: AbortSignal,
  options: PageSceneOptions,
  browser = { keys: browserFrameKeys, load: loadBrowserScene },
): Promise<{ results: FrameResult[]; fallback: string }> {
  const results: FrameResult[] = [];
  const failed = (cause: unknown) =>
    `${WASM_FALLBACK}: ${cause instanceof Error ? cause.message : "Browser computation failed."}`;
  let digests: Map<string, string>;
  try {
    digests = await browser.keys(requests, snapshot, signal);
  } catch (cause) {
    signal.throwIfAborted();
    return { results, fallback: failed(cause) };
  }
  for (const request of requests) {
    const digest = digests.get(request.frameId) ?? "";
    const contentKey = frameContentKey(request, digest);
    // Unchanged frames already on screen cost nothing, not even a frame yield.
    if (
      contentKey &&
      !options.bypassCache &&
      options.reuse?.(request, contentKey)
    ) {
      results.push({ request, contentKey, reused: true });
      continue;
    }
    await yieldFrame(signal);
    let result: FrameResult;
    try {
      result = {
        request,
        contentKey,
        ...(await browser.load(
          request,
          snapshot,
          signal,
          options.bypassCache,
          digest,
        )),
      };
    } catch (cause) {
      signal.throwIfAborted();
      return { results, fallback: failed(cause) };
    }
    await options.onFrame?.(result, signal);
    signal.throwIfAborted();
    results.push(result);
  }
  return { results, fallback: "" };
}
