import { ApiError, projectPath, viewerFetch } from "./api.ts";
import type { Snapshot } from "./types";

/** In-memory validator for exactly one accepted object-format representation. */
export interface AcceptedSnapshot {
  snapshot: Snapshot;
  etag: string | null;
}

function strongEtag(value: string | null): string | null {
  return value && /^"[\x21\x23-\x7e\x80-\xff]*"$/.test(value) ? value : null;
}

/** A null result is a foreign-project or backwards-revision response to ignore. */
export async function readSnapshot(
  projectId: string,
  current: AcceptedSnapshot | null,
  signal: AbortSignal,
): Promise<AcceptedSnapshot | null> {
  signal.throwIfAborted();
  const previous = current?.snapshot.projectId === projectId ? current : null;
  const etag = strongEtag(previous?.etag ?? null);
  const headers: Record<string, string> = { Accept: "application/json" };
  if (etag) headers["If-None-Match"] = etag;
  const response = await viewerFetch(`${projectPath(projectId)}/snapshot`, {
    signal,
    headers,
    // The validator above belongs to the live hook, never the browser HTTP cache.
    cache: "no-store",
  });
  signal.throwIfAborted();
  if (response.status === 304) {
    const responseEtag = response.headers.get("ETag");
    if (!previous || !etag || (responseEtag && responseEtag !== etag))
      throw new ApiError(
        "INVALID_SNAPSHOT_RESPONSE",
        "Could not validate this snapshot. Retry this view.",
        response.status,
      );
    return previous;
  }
  const body = await response.json();
  signal.throwIfAborted();
  if (!response.ok)
    throw new ApiError(
      body.error?.code ?? "HTTP_ERROR",
      body.error?.message ?? "Could not load this view.",
      response.status,
    );
  const next = body as Snapshot;
  if (next.projectId !== projectId) return null;
  if (next.truncated)
    throw new Error(
      "This snapshot is incomplete. Retry or reduce the project size.",
    );
  if (previous && !(next.revision >= previous.snapshot.revision)) return null;
  // Equal authored revisions can still contain new privacy/resource/metadata data.
  return { snapshot: next, etag: strongEtag(response.headers.get("ETag")) };
}
