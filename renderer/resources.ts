import { type AssetManifest, type FontManifest, type Resources } from "./types";
import { PipelineError } from "./error";
import { fontCSS, fontFamily } from "./font-family";
import { parseCoverage } from "./glyph-coverage";

const sha256 = async (bytes: ArrayBuffer) =>
  Array.from(
    new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)),
    (x) => x.toString(16).padStart(2, "0"),
  ).join("");

async function bounded<T>(promise: Promise<T>, timeoutMs: number): Promise<T> {
  let timer: ReturnType<typeof setTimeout>;
  try {
    return await Promise.race([
      promise,
      new Promise<never>((_, reject) => {
        timer = setTimeout(
          () =>
            reject(
              new PipelineError(
                "RESOURCE_TIMEOUT",
                "Resource loading timed out",
              ),
            ),
          timeoutMs,
        );
      }),
    ]);
  } finally {
    clearTimeout(timer!);
  }
}

async function checkedBytes(
  resource: FontManifest | AssetManifest,
  timeoutMs: number,
): Promise<ArrayBuffer> {
  let response: Response;
  try {
    response = await fetch(resource.url, {
      signal: AbortSignal.timeout(timeoutMs),
    });
  } catch {
    throw new PipelineError(
      "RESOURCE_UNAVAILABLE",
      `Resource ${resource.id} unavailable`,
    );
  }
  if (!response.ok)
    throw new PipelineError(
      "RESOURCE_UNAVAILABLE",
      `Resource ${resource.id} unavailable`,
    );
  const bytes = await response.arrayBuffer();
  if ((await sha256(bytes)) !== resource.hash)
    throw new PipelineError(
      "RESOURCE_HASH_MISMATCH",
      `Resource ${resource.id} hash mismatch`,
    );
  return bytes;
}

/** Explicit bytes + SHA and private CSS families. Readiness does not prove glyph coverage. */
export async function loadResources(
  fonts: FontManifest[],
  assets: AssetManifest[],
  options: { strict?: boolean; timeoutMs?: number } = {},
): Promise<Resources> {
  const result: Resources = {
    fonts: new Map(),
    images: new Map(),
    assetHashes: new Map(),
    readiness: [],
    diagnostics: [],
    glyphCoverage: new Map(),
  };
  const timeoutMs = options.timeoutMs ?? 10_000;
  await Promise.all([
    ...fonts.map(async (font) => {
      try {
        const bytes = await checkedBytes(font, timeoutMs);
        const family = fontFamily(font);
        const face = new FontFace(family, bytes, {
          weight: String(font.weight),
          style: font.style,
        });
        await bounded(face.load(), timeoutMs);
        for (const previous of document.fonts) {
          if (
            previous.family === family &&
            previous.weight === String(font.weight) &&
            previous.style === font.style
          )
            document.fonts.delete(previous);
        }
        document.fonts.add(face);
        await bounded(
          document.fonts.load(
            fontCSS(font, 16),
            "Tiếng Việt ă â đ ê ô ơ ư",
          ),
          timeoutMs,
        );
        if (!document.fonts.check(fontCSS(font, 16)))
          throw new PipelineError("FONT_MISSING", `Font ${font.id} not ready`);
        result.fonts.set(font.id, font);
        const coverage = parseCoverage(bytes);
        if (coverage) result.glyphCoverage!.set(font.id, coverage);
        result.readiness.push({ id: font.id, kind: "font", status: "ready" });
      } catch (e) {
        result.readiness.push({
          id: font.id,
          kind: "font",
          status: "failed",
          code: e instanceof PipelineError ? e.code : "FONT_MISSING",
        });
        result.diagnostics.push(`Font ${font.id} failed readiness`);
      }
    }),
    ...assets.map(async (asset) => {
      try {
        const bytes = await checkedBytes(asset, timeoutMs);
        const image = new Image();
        const url = URL.createObjectURL(
          new Blob([bytes], { type: asset.mediaType }),
        );
        try {
          image.src = url;
          await bounded(image.decode(), timeoutMs);
        } finally {
          URL.revokeObjectURL(url);
        }
        result.images.set(asset.id, image);
        result.assetHashes.set(asset.id, asset.hash);
        result.readiness.push({ id: asset.id, kind: "asset", status: "ready" });
      } catch (e) {
        result.readiness.push({
          id: asset.id,
          kind: "asset",
          status: "failed",
          code: e instanceof PipelineError ? e.code : "ASSET_DECODE_FAILED",
        });
        result.diagnostics.push(`Asset ${asset.id} failed readiness`);
      }
    }),
  ]);
  await bounded(document.fonts.ready, timeoutMs);
  result.readiness.sort(
    (a, b) => a.kind.localeCompare(b.kind) || a.id.localeCompare(b.id),
  );
  if (options.strict !== false && result.diagnostics.length)
    throw new PipelineError(
      "RESOURCE_NOT_READY",
      result.diagnostics.join("; "),
    );
  return result;
}
