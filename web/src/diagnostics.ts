/** Compact display text; the structured evidence stays in the scene/job response. */
export function diagnosticLabel(value: unknown): string {
  if (typeof value === "string") return value;
  if (!value || typeof value !== "object") return "Design diagnostic";
  const issue = value as {
    code?: unknown;
    message?: unknown;
    nodeId?: unknown;
  };
  const code =
    typeof issue.code === "string" ? issue.code : "Design diagnostic";
  const node = typeof issue.nodeId === "string" ? ` (${issue.nodeId})` : "";
  const message = typeof issue.message === "string" ? `: ${issue.message}` : "";
  return `${code}${node}${message}`;
}
