import { ApiError, projectPath, read } from "../api";
import type { FontImport, FontList, ProjectFont } from "./types";

export const listFonts = (projectId: string, signal: AbortSignal) =>
  read<FontList>(`${projectPath(projectId)}/fonts`, signal);

export class FontImportError extends ApiError {
  constructor(
    code: string,
    message: string,
    public ambiguous: boolean,
  ) {
    super(code, message);
  }
}

export async function importFont(
  projectId: string,
  request: FontImport,
  signal: AbortSignal,
) {
  const response = await fetch(`${projectPath(projectId)}/fonts`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify(request),
    signal: AbortSignal.any([signal, AbortSignal.timeout(30000)]),
  });
  const result = await response.json();
  if (!response.ok)
    throw new FontImportError(
      result.error?.code ?? "IMPORT_FAILED",
      result.error?.message ?? "The font could not be added.",
      response.status >= 500,
    );
  return result as { committedRevision: number };
}

/** Fetch first so a missing font produces a visible error, never a JSON download. */
export async function downloadFont(
  projectId: string,
  font: ProjectFont,
  signal: AbortSignal,
) {
  const response = await fetch(
    `${projectPath(projectId)}/fonts/${encodeURIComponent(font.id)}/download`,
    { signal },
  );
  if (!response.ok)
    throw new Error(
      "The font file is unavailable. Refresh the font list and try again.",
    );
  const disposition = response.headers.get("Content-Disposition") ?? "";
  const fileName =
    disposition.match(/filename="([^"]+)"/)?.[1] ??
    `${font.family}.${font.mediaType === "font/woff2" ? "woff2" : font.mediaType === "font/otf" ? "otf" : "ttf"}`;
  const url = URL.createObjectURL(await response.blob());
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = fileName;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
