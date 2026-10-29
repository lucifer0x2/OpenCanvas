import { useEffect, useState } from "react";
import { loadResources } from "../../../renderer/index";
import type { Resources, Scene } from "../../../renderer/types";
import { projectPath, read } from "../api";
import { fetchSceneJob } from "../sceneJob";
import { declaredStates, frameSize, rootFrames } from "./model";
import type { PreviewSnapshot, StateName } from "./types";

export interface PreviewScenes {
  status: "loading" | "ready" | "failed";
  scene?: Scene;
  states: Partial<Record<StateName, Scene>>;
  resources?: Resources;
  diagnostics: string[];
  error?: string;
}

export interface PreviewTarget {
  frameId: string;
  width?: number;
  height?: number;
}

const LOADING: PreviewScenes = {
  status: "loading",
  states: {},
  diagnostics: [],
};

/** The authored snapshot of one project (GET only), refetched when `reload` changes. */
export function usePreviewSnapshot(projectId: string, reload: number) {
  const [snapshot, setSnapshot] = useState<PreviewSnapshot | null>(null);
  const [error, setError] = useState("");
  useEffect(() => {
    setSnapshot(null);
    setError("");
    if (!projectId) {
      setError("Add ?project=<projectId> to the URL.");
      return;
    }
    const controller = new AbortController();
    read<PreviewSnapshot>(
      `${projectPath(projectId)}/snapshot`,
      controller.signal,
    )
      .then((next) => {
        if (next.truncated) throw new Error("This snapshot is incomplete.");
        setSnapshot(next);
      })
      .catch((cause: Error) => {
        if (!controller.signal.aborted) setError(cause.message);
      });
    return () => controller.abort();
  }, [projectId, reload]);
  return { snapshot, error };
}

/**
 * The default scene of the frame plus one scene per interaction state declared in it, and the
 * default scene's fonts and images (fonts register in `document.fonts`).
 */
export function usePreviewScenes(
  snapshot: PreviewSnapshot | null,
  target: PreviewTarget,
): PreviewScenes {
  const [view, setView] = useState<PreviewScenes>(LOADING);
  const frame = snapshot
    ? rootFrames(snapshot.nodes).find((node) => node.id === target.frameId)
    : undefined;
  const size = frame ? frameSize(frame) : undefined;
  const width = target.width ?? size?.width ?? 0;
  const height = target.height ?? size?.height ?? 0;
  const key = snapshot
    ? `${snapshot.projectId}:${snapshot.revision}:${target.frameId}:${width}x${height}`
    : "";
  useEffect(() => {
    if (!snapshot || !frame) {
      setView(
        snapshot
          ? { ...LOADING, status: "failed", error: "Frame not found." }
          : LOADING,
      );
      return;
    }
    const controller = new AbortController();
    const request = {
      projectId: snapshot.projectId,
      revision: snapshot.revision,
      frameId: frame.id,
      width,
      height,
    };
    setView(LOADING);
    const names = declaredStates(snapshot.nodes, frame.id);
    const load = async () => {
      const [base, ...variants] = await Promise.all([
        fetchSceneJob(request, controller.signal),
        ...names.map((name) => fetchSceneJob(request, controller.signal, name)),
      ]);
      const scene = base.scene!;
      const resources = await loadResources(scene.fonts, scene.assets, {
        strict: false,
      });
      if (controller.signal.aborted) return;
      const states: PreviewScenes["states"] = {};
      names.forEach((name, i) => (states[name] = variants[i].scene));
      setView({
        status: "ready",
        scene,
        states,
        resources,
        diagnostics: [
          ...new Set([...scene.diagnostics, ...resources.diagnostics]),
        ],
      });
    };
    load().catch((cause: Error) => {
      if (!controller.signal.aborted)
        setView({ ...LOADING, status: "failed", error: cause.message });
    });
    return () => controller.abort();
    // `key` captures snapshot revision, frame and size.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [key]);
  return view;
}
