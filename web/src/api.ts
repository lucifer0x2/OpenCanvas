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

/** Viewer intentionally exposes only GET. Design changes belong to the agent's MCP client. */
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
