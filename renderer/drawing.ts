import {
  MAX_PIXELS,
  RENDERER_VERSION,
  SCENE_VERSION,
  type Resources,
  type Scene,
} from "./types";
import { PipelineError } from "./error";
import { paintScene } from "./paint";

/** The worker PNG and the viewer's 1:1 parity canvas: validate, size the target, then paint. */
export function drawScene(
  target: HTMLCanvasElement,
  scene: Scene,
  resources: Resources,
  options: { scale?: number; strict?: boolean } = {},
): { sourceSceneKey: string; drawnLines: number } {
  if (
    scene.provenance.rendererVersion !== RENDERER_VERSION ||
    scene.provenance.sceneVersion !== SCENE_VERSION
  )
    throw new PipelineError(
      "PIPELINE_MISMATCH",
      "Scene pipeline version mismatch",
    );
  if (
    scene.fonts.some(
      (font) =>
        resources.fonts.has(font.id) &&
        resources.fonts.get(font.id)?.hash !== font.hash,
    ) ||
    scene.assets.some(
      (asset) =>
        resources.assetHashes.has(asset.id) &&
        resources.assetHashes.get(asset.id) !== asset.hash,
    )
  )
    throw new PipelineError("PIPELINE_MISMATCH", "Resource manifest mismatch");
  const scale = options.scale ?? 1;
  const viewport = scene.provenance.viewport;
  if (
    !Number.isFinite(scale) ||
    scale <= 0 ||
    scale > 4 ||
    Math.ceil(viewport.width * scale) * Math.ceil(viewport.height * scale) >
      MAX_PIXELS
  )
    throw new PipelineError("PIXEL_BUDGET", "Render pixel budget exceeded");
  if (
    options.strict !== false &&
    (resources.diagnostics.length ||
      scene.nodes.some(
        (n) => n.kind === "text" && !resources.fonts.has(n.fontId ?? ""),
      ) ||
      scene.nodes.some(
        (n) => n.kind === "image" && !resources.images.has(n.assetId ?? ""),
      ))
  )
    throw new PipelineError(
      "RESOURCE_NOT_READY",
      "Scene resources are not ready",
    );
  target.width = Math.ceil(viewport.width * scale);
  target.height = Math.ceil(viewport.height * scale);
  const context = target.getContext("2d");
  if (!context)
    throw new PipelineError("CANVAS_UNAVAILABLE", "Canvas2D unavailable");
  context.setTransform(scale, 0, 0, scale, 0, 0);
  context.clearRect(0, 0, viewport.width, viewport.height);
  paintScene(context, scene, resources);
  return {
    sourceSceneKey: scene.provenance.sourceSceneKey,
    drawnLines: scene.nodes.reduce((count, n) => count + n.lines.length, 0),
  };
}
