import type {
  AssetManifest,
  FontManifest,
  Scene,
} from "../../../renderer/types.ts";
import { projectPath } from "../api.ts";
import type { SceneRequest, Snapshot } from "../types.ts";
import type { ResourceURLs } from "./protocol.ts";

export function snapshotResources(snapshot: Snapshot, request: SceneRequest) {
  if (
    snapshot.projectId !== request.projectId ||
    snapshot.revision !== request.revision ||
    snapshot.truncated
  )
    throw new Error(
      "Browser scene requires the accepted snapshot of this revision.",
    );
  if (
    ![
      snapshot.nodes,
      snapshot.tokens,
      snapshot.styles,
      snapshot.fonts,
      snapshot.assets,
    ].every(Array.isArray)
  )
    throw new Error("Browser scene requires complete authored collections.");
  const path = projectPath(snapshot.projectId);
  const fonts: FontManifest[] = snapshot.fonts
    .filter((font) => font.contentHash !== null)
    .map((font) => ({
      id: font.id,
      family: font.family,
      hash: font.contentHash!,
      weight: font.weight,
      style: font.style,
      url: `${path}/fonts/${encodeURIComponent(font.id)}/download`,
    }));
  const assets: AssetManifest[] = snapshot.assets.map((asset) => ({
    id: asset.id,
    hash: asset.contentHash,
    mediaType: asset.mediaType,
    url: `${path}/assets/${encodeURIComponent(asset.id)}`,
  }));
  const urls: ResourceURLs = {
    fonts: Object.fromEntries(fonts.map((font) => [font.id, font.url])),
    assets: Object.fromEntries(assets.map((asset) => [asset.id, asset.url])),
  };
  return { fonts, assets, urls };
}

export function decorateFonts(
  fonts: FontManifest[],
  expected: ReturnType<typeof snapshotResources>,
): FontManifest[] {
  return fonts.map((font) => {
    const found = expected.fonts.find((item) => item.id === font.id);
    if (
      !found ||
      found.hash !== font.hash ||
      found.family !== font.family ||
      found.weight !== font.weight ||
      found.style !== font.style
    )
      throw new Error(
        "Browser scene font manifest does not match its snapshot.",
      );
    return { ...font, url: found.url };
  });
}

/** A compiler may emit no transport URL; only this API adapter supplies controlled paths. */
export function decorateResources(
  scene: Scene,
  expected: ReturnType<typeof snapshotResources>,
): Scene {
  const fonts = decorateFonts(scene.fonts, expected);
  const assets = scene.assets.map((asset) => {
    const found = expected.assets.find((item) => item.id === asset.id);
    if (
      !found ||
      found.hash !== asset.hash ||
      found.mediaType !== asset.mediaType
    )
      throw new Error(
        "Browser scene asset manifest does not match its snapshot.",
      );
    return { ...asset, url: found.url };
  });
  return { ...scene, fonts, assets };
}
