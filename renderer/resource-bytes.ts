import { PipelineError } from "./error.ts";
import type { AssetManifest, FontManifest } from "./types.ts";

/** Fetch and validate immutable resource bytes before either measurement or painting. */
export async function checkedResourceBytes(
  resource: FontManifest | AssetManifest,
  timeoutMs = 10_000,
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
  const hash = Array.from(
    new Uint8Array(await crypto.subtle.digest("SHA-256", bytes)),
    (value) => value.toString(16).padStart(2, "0"),
  ).join("");
  if (hash !== resource.hash)
    throw new PipelineError(
      "RESOURCE_HASH_MISMATCH",
      `Resource ${resource.id} hash mismatch`,
    );
  return bytes;
}

/** Providers must return bytes already validated against this exact manifest's SHA-256. */
export type FontBytesProvider = (
  font: FontManifest,
  timeoutMs: number,
) => Promise<ArrayBuffer>;
