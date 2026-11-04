const AUTH_PATH = "/api/v1/viewer/auth";
const STATUS_ERROR = "Could not check viewer access. Try again.";

interface ViewerAuth {
  authenticated: boolean;
  required: boolean;
}

export class ViewerLoginError extends Error {
  constructor(
    message: string,
    public readonly incorrectPassword = false,
  ) {
    super(message);
  }
}

export async function viewerAuth(signal: AbortSignal): Promise<ViewerAuth> {
  const response = await fetch(AUTH_PATH, {
    cache: "no-store",
    signal: AbortSignal.any([signal, AbortSignal.timeout(10_000)]),
  });
  if (!response.ok) throw new Error(STATUS_ERROR);
  const { authenticated, required } = await response.json();
  if (typeof authenticated !== "boolean" || typeof required !== "boolean")
    throw new Error(STATUS_ERROR);
  return { authenticated, required };
}

export async function unlockViewer(password: string, signal: AbortSignal) {
  const response = await fetch(AUTH_PATH, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ password }),
    signal: AbortSignal.any([signal, AbortSignal.timeout(15_000)]),
  });
  if (response.status === 401)
    throw new ViewerLoginError("Incorrect password. Try again.", true);
  if (response.status === 429)
    throw new ViewerLoginError(
      "Too many attempts. Wait a moment and try again.",
    );
  if (!response.ok || (await response.json()).authenticated !== true)
    throw new Error("Viewer login failed.");
  const access = await viewerAuth(signal);
  if (!access.authenticated) throw new Error("Viewer session unavailable.");
  return access.required;
}
