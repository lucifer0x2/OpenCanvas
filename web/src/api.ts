export class ApiError extends Error {
  constructor(
    public code: string,
    message: string,
  ) {
    super(message);
  }
}

export const projectPath = (projectId: string) =>
  `/api/v1/projects/${encodeURIComponent(projectId)}`;

/** Read viewer data. Font resource imports are the explicit UI mutation exception. */
export async function read<T>(path: string, signal?: AbortSignal): Promise<T> {
  const response = await fetch(path, {
    signal,
    headers: { Accept: "application/json" },
  });
  const body = await response.json();
  if (!response.ok)
    throw new ApiError(
      body.error?.code ?? "HTTP_ERROR",
      body.error?.message ?? "Could not load this view.",
    );
  return body as T;
}
