import { ApiError, projectPath, read } from "./api";

interface ExportFile {
  url: string;
  fileName: string;
}
interface ExportItem {
  jobId: string;
  status: "pending" | "ready" | "failed";
  files?: ExportFile[];
  error?: { code: string; message: string };
}

/** Export is bounded computed work; it never writes design data. */
export async function exportNode(
  projectId: string,
  nodeId: string,
  revision: number,
  format: "png" | "svg",
) {
  const response = await fetch(`${projectPath(projectId)}/exports`, {
    method: "POST",
    headers: { "Content-Type": "application/json", Accept: "application/json" },
    body: JSON.stringify({
      nodeIds: [nodeId],
      revision,
      settings: [{ format }],
      waitMs: 10000,
    }),
    signal: AbortSignal.timeout(15000),
  });
  const result = await response.json();
  if (!response.ok)
    throw new ApiError(
      result.error?.code ?? "EXPORT_FAILED",
      result.error?.message ?? "Export failed.",
    );
  let item: ExportItem = result.exports[0];
  const deadline = Date.now() + 60000;
  while (item.status === "pending" && Date.now() < deadline) {
    await new Promise((resolve) => setTimeout(resolve, 500));
    item = await read<ExportItem>(
      `/api/v1/jobs/${encodeURIComponent(item.jobId)}`,
    );
  }
  if (item.status !== "ready")
    throw new Error(
      item.error?.message ?? "Export is still pending. Try again shortly.",
    );
  const file = item.files?.[0];
  if (!file) throw new Error("Export returned no file.");
  // Fetch first so failed/evicted jobs show an error, rather than downloading JSON as an image.
  const filePath = new URL(file.url, location.href).pathname;
  if (!/^\/api\/v1\/jobs\/[^/]+\/files\/\d+$/.test(filePath))
    throw new Error("Invalid export file URL.");
  const download = await fetch(filePath, {
    signal: AbortSignal.timeout(15000),
  });
  if (!download.ok)
    throw new Error("The export file is no longer available. Export again.");
  const url = URL.createObjectURL(await download.blob());
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = file.fileName;
  anchor.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
