import {
  MAX_PIXELS,
  RENDERER_VERSION,
  SCENE_VERSION,
  type FontManifest,
  type Resources,
  type Scene,
  type SceneNode,
} from "./types";
import { PipelineError } from "./error";
import { placeImage } from "./image-fit";

const fontCSS = (font: FontManifest, size: number) =>
  `${font.style} ${font.weight} ${size}px "${font.family}"`;
function paintNode(
  context: CanvasRenderingContext2D,
  node: SceneNode,
  resources: Resources,
) {
  const b = node.bounds;
  if (node.fill !== "transparent" && node.kind !== "text") {
    context.fillStyle = node.fill;
    context.beginPath();
    context.roundRect(b.x, b.y, b.width, b.height, node.radius ?? 0);
    context.fill();
  }
  if (node.kind === "text") {
    const font = resources.fonts.get(node.fontId ?? "");
    if (!font) {
      context.fillStyle = "#be123c";
      context.font = "12px sans-serif";
      context.fillText("[Missing font]", b.x, b.y + 14);
      return;
    }
    context.font = fontCSS(font, node.fontSize);
    context.fontKerning = "normal";
    context.textBaseline = "alphabetic";
    context.fillStyle = node.fill;
    // Do not call measureText, wrap, use maxWidth, or Konva.Text here.
    for (const line of node.lines)
      context.fillText(line.text, b.x + line.x, b.y + line.y);
  }
  if (node.kind === "image") {
    const image = resources.images.get(node.assetId ?? "");
    if (!image) {
      context.fillStyle = "#ffe4e6";
      context.fillRect(b.x, b.y, b.width, b.height);
      context.fillStyle = "#9f1239";
      context.font = "12px sans-serif";
      context.fillText("[Missing image]", b.x + 4, b.y + 16);
      return;
    }
    context.save();
    context.beginPath();
    context.roundRect(b.x, b.y, b.width, b.height, node.radius ?? 0);
    context.clip();
    const crop = node.crop ?? { x: 0, y: 0, width: 1, height: 1 };
    const sw = image.naturalWidth * crop.width,
      sh = image.naturalHeight * crop.height;
    const at = placeImage(node.fit, b, { width: sw, height: sh });
    context.drawImage(
      image,
      image.naturalWidth * crop.x,
      image.naturalHeight * crop.y,
      sw,
      sh,
      b.x + at.x,
      b.y + at.y,
      at.width,
      at.height,
    );
    context.restore();
  }
}

/** Shared by the browser worker PNG and a viewer's Konva.Image source canvas. No layout/rewrap. */
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
  const byId = new Map(scene.nodes.map((n) => [n.id, n]));
  for (const node of scene.nodes) {
    context.save();
    let opacity = node.opacity;
    let parent = node.parentId ? byId.get(node.parentId) : undefined;
    let guard = 0;
    while (parent) {
      if (++guard > scene.nodes.length)
        throw new PipelineError("SCENE_INVALID", "Scene cycle");
      opacity *= parent.opacity;
      if (parent.clip) {
        const b = parent.bounds;
        context.beginPath();
        context.roundRect(b.x, b.y, b.width, b.height, parent.radius ?? 0);
        context.clip();
      }
      parent = parent.parentId ? byId.get(parent.parentId) : undefined;
    }
    context.globalAlpha = opacity;
    paintNode(context, node, resources);
    context.restore();
  }
  return {
    sourceSceneKey: scene.provenance.sourceSceneKey,
    drawnLines: scene.nodes.reduce((count, n) => count + n.lines.length, 0),
  };
}
