export class ApiError extends Error {
  constructor(
    public code: string,
    message: string,
    public status?: number,
  ) {
    super(message);
  }
}

export const projectPath = (projectId: string) =>
  `/api/v1/projects/${encodeURIComponent(projectId)}`;

export const VIEWER_SESSION_CHECK = "opencanvas:viewer-session-check";

export function requestViewerSessionCheck() {
  window.dispatchEvent(new Event(VIEWER_SESSION_CHECK));
}

export async function viewerFetch(path: string, init?: RequestInit) {
  const response = await fetch(path, init);
  if (response.status === 401 && !init?.signal?.aborted)
    requestViewerSessionCheck();
  return response;
}

/** Read viewer data. Font resource imports are the explicit UI mutation exception. */
export async function read<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await viewerFetch(path, {
    signal,
    headers: { Accept: "application/json" },
  });
  const body = await response.json();
  if (!response.ok)
    throw new ApiError(
      body.error?.code ?? "HTTP_ERROR",
      body.error?.message ?? "Could not load this view.",
      response.status,
    );
  return body as T;
}
