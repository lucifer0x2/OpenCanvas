import type { DesignNode } from "./types.ts";

export interface NodeIndex {
  byId: ReadonlyMap<string, DesignNode>;
  childrenByParent: ReadonlyMap<string | null, readonly DesignNode[]>;
}

const indexes = new WeakMap<DesignNode[], NodeIndex>();
export const EMPTY_NODES: DesignNode[] = [];

/** Snapshot node arrays are immutable; release their index with the snapshot. */
export function nodeIndex(nodes: DesignNode[]): NodeIndex {
  const cached = indexes.get(nodes);
  if (cached) return cached;
  const byId = new Map<string, DesignNode>();
  const childrenByParent = new Map<string | null, DesignNode[]>();
  for (const node of nodes) {
    byId.set(node.id, node);
    const siblings = childrenByParent.get(node.parentId);
    if (siblings) siblings.push(node);
    else childrenByParent.set(node.parentId, [node]);
  }
  for (const siblings of childrenByParent.values()) {
    siblings.sort(
      (a, b) => Number(a.sortKey) - Number(b.sortKey) ||
        (a.id < b.id ? -1 : a.id > b.id ? 1 : 0),
    );
  }
  const index = { byId, childrenByParent };
  indexes.set(nodes, index);
  return index;
}
