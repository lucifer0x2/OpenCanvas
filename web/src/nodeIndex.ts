import type { DesignNode, Snapshot } from "./types.ts";

export interface NodeIndex {
  byId: ReadonlyMap<string, DesignNode>;
  childrenByParent: ReadonlyMap<string | null, readonly DesignNode[]>;
}

const indexes = new WeakMap<DesignNode[], NodeIndex>();
// Retain only the newest authored revision per visited project until this tab reloads.
// Array bindings below are auxiliary and disappear when old snapshots are released.
const projects = new Map<string, { revision: number; index: NodeIndex }>();
export const EMPTY_NODES: DesignNode[] = [];

/** Array-only helpers and standalone/subset callers share immutable node arrays. */
export function nodeIndex(nodes: DesignNode[]): NodeIndex {
  const cached = indexes.get(nodes);
  if (cached) return cached;
  return buildNodeIndex(nodes);
}

function buildNodeIndex(nodes: DesignNode[]): NodeIndex {
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

/** Authored nodes change only with a project's revision, even across JSON refetches. */
export function snapshotNodeIndex(
  snapshot: Pick<Snapshot, "projectId" | "revision" | "nodes">,
): NodeIndex {
  const cached = projects.get(snapshot.projectId);
  if (cached?.revision === snapshot.revision) {
    indexes.set(snapshot.nodes, cached.index);
    return cached.index;
  }
  const index = cached && snapshot.revision > cached.revision
    ? buildNodeIndex(snapshot.nodes)
    : nodeIndex(snapshot.nodes);
  // Obsolete renders may still consult their old snapshot; never downgrade the project.
  if (!cached || snapshot.revision > cached.revision) {
    projects.set(snapshot.projectId, { revision: snapshot.revision, index });
  }
  return index;
}
