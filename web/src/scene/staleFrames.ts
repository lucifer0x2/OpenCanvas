import type { FrameSceneView, SceneView } from "../hooks/useScene";
import type { SceneRequest } from "../types";
import { rebaseScene } from "./sceneIdentity";

export type FrameTarget = Pick<
  FrameSceneView,
  "id" | "name" | "width" | "height"
>;

/**
 * Stale-while-revalidate: each target keeps the last scene of the same frame, marked stale,
 * so a new revision never blanks rendered frames. Frames new to this revision start empty;
 * frames no longer authored are dropped.
 */
export function seedFrames(
  targets: FrameTarget[],
  previous: FrameSceneView[] = [],
): FrameSceneView[] {
  const byId = new Map(previous.map((frame) => [frame.id, frame]));
  return targets.map((target) => {
    const old = byId.get(target.id);
    return old?.scene && old.resources
      ? {
          ...target,
          scene: old.scene,
          resources: old.resources,
          contentKey: old.contentKey,
          diagnostics: old.diagnostics,
          missing: old.missing,
          stale: true,
        }
      : { ...target };
  });
}

/** A stale frame with the same content key is current again without loading anything. */
export function reuseFrame(
  entry: FrameSceneView | undefined,
  request: SceneRequest,
  contentKey: string,
): FrameSceneView | undefined {
  if (
    !entry?.stale ||
    !entry.scene ||
    !entry.resources ||
    !contentKey ||
    entry.contentKey !== contentKey
  )
    return;
  const { stale: _stale, updateError: _error, ...current } = entry;
  return { ...current, scene: rebaseScene(entry.scene, request) };
}

/** A failed frame keeps its last scene, if any, and reports the failure beside it. */
export function failFrame(
  entry: FrameSceneView,
  target: FrameTarget,
  error: string,
): FrameSceneView {
  return entry.scene ? { ...entry, updateError: error } : { ...target, error };
}

/** Aggregate one page; `seeded` means earlier scenes are on screen, so progress is quiet. */
export function pageView(
  entries: FrameSceneView[],
  options: { finished?: boolean; error?: string; seeded?: boolean },
): SceneView {
  const { finished = false, error, seeded = false } = options;
  const failures = entries.filter(
    (entry) => entry.error || entry.updateError,
  ).length;
  const shown = entries.find((entry) => entry.scene);
  const status: SceneView["status"] =
    error || (finished && failures)
      ? "failed"
      : !entries.length
        ? "empty"
        : finished
          ? "ready"
          : "pending";
  return {
    status,
    frames: [...entries],
    scene: shown?.scene,
    resources: shown?.resources,
    diagnostics: [
      ...new Set(entries.flatMap((entry) => entry.diagnostics ?? [])),
    ],
    missing: [...new Set(entries.flatMap((entry) => entry.missing ?? []))],
    progress: {
      ready: entries.filter((entry) => entry.scene && !entry.stale).length,
      total: entries.length,
    },
    revalidating: seeded && status === "pending",
    error:
      error ??
      (finished && failures
        ? `${failures} of ${entries.length} frames failed. Retry to render the complete page.`
        : undefined),
  };
}
