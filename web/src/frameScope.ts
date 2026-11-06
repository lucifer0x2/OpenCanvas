import { selectionNode } from "./deepLink.ts";
import type { DesignNode } from "./types.ts";

/** Persisted picker value; never sent as a frame ID to a render endpoint. */
export const ALL_FRAMES = ":all:";

export function frameForNode(
  nodes: DesignNode[],
  nodeId: string,
  pageId: string,
) {
  const byId = new Map(nodes.map((node) => [node.id, node]));
  let node = selectionNode(nodes, nodeId);
  const seen = new Set<string>();
  while (node && !seen.has(node.id)) {
    if (node.kind === "frame" && node.parentId === pageId) return node;
    seen.add(node.id);
    node =
      byId.get(node.parentId ?? "") ??
      selectionNode(nodes, node.parentId ?? "");
  }
}

export function frameDimensions(node: DesignNode) {
  const dimension = (value: unknown, fallback: number) =>
    typeof value === "number" && Number.isFinite(value) && value > 0
      ? value
      : fallback;
  return {
    width: dimension(node.props.width, 800),
    height: dimension(node.props.height, 600),
  };
}
