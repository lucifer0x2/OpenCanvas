import { selectionNode } from "./deepLink.ts";
import { nodeIndex } from "./nodeIndex.ts";
import type { DesignNode } from "./types.ts";

export interface LayerSelectionPath {
  id: string;
  child?: LayerSelectionPath;
}

/** Only the selected branch receives a changing prop; unrelated expanded rows stay idle. */
export function layerSelectionPath(nodes: DesignNode[], selectedId: string) {
  const { byId } = nodeIndex(nodes);
  let node = selectionNode(nodes, selectedId);
  let path: LayerSelectionPath | undefined;
  const seen = new Set<string>();
  while (node && !seen.has(node.id)) {
    seen.add(node.id);
    path = { id: node.id, child: path };
    const parentId = node.parentId ?? "";
    node = byId.get(parentId) ?? selectionNode(nodes, parentId);
    // Pages aren't rows in the layers panel.
    if (node?.kind === "page") break;
  }
  return path;
}
