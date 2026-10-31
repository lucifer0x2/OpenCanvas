/** Compact display text; the structured evidence stays in the scene/job response. */
export function diagnosticLabel(value: unknown): string {
  if (typeof value === "string") return value;
  if (!value || typeof value !== "object") return "Design diagnostic";
  const issue = value as {
    code?: unknown;
    message?: unknown;
    nodeId?: unknown;
    reportedNodes?: unknown;
    omittedNodes?: unknown;
  };
  const code =
    typeof issue.code === "string" ? issue.code : "Design diagnostic";
  const node = typeof issue.nodeId === "string" ? ` (${issue.nodeId})` : "";
  const message = typeof issue.message === "string" ? `: ${issue.message}` : "";
  // Omission summaries carry counts (and provenance); only the counts are shown.
  const counts =
    typeof issue.reportedNodes === "number" &&
    typeof issue.omittedNodes === "number"
      ? `: ${issue.reportedNodes} reported, ${issue.omittedNodes} omitted`
      : "";
  return `${code}${node}${message || counts}`;
}
