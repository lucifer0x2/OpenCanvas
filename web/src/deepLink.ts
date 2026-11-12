import type { DesignNode, Selection } from "./types.ts";
import { nodeIndex } from "./nodeIndex.ts";

const KEYS = ["projectId", "pageId", "frameId", "nodeId"] as const;
const PARAMS = ["project", "page", "frame", "node"] as const;

/** A design address follows stable IDs at the latest revision, independent of viewport. */
export function readNodeLink(href: string): Selection | null {
  const url = new URL(href);
  if (!PARAMS.slice(0, 3).every((key) => url.searchParams.get(key)))
    return null;
  return readNavigation(href);
}

/** Partial addresses are retained until authoritative data can supply defaults. */
export function readNavigation(href: string): Selection {
  const url = new URL(href);
  return Object.fromEntries(
    KEYS.map((key, i) => [key, url.searchParams.get(PARAMS[i]) ?? ""]),
  ) as unknown as Selection;
}

/** Address-bar updates preserve unrelated query parameters and the fragment. */
export function navigationLink(href: string, selection: Selection): string {
  const url = new URL(href);
  KEYS.forEach((key, i) => {
    if (selection.projectId && selection[key])
      url.searchParams.set(PARAMS[i], selection[key]);
    else url.searchParams.delete(PARAMS[i]);
  });
  return url.href;
}

export function nodeLink(href: string, selection: Selection): string {
  return navigationLink(new URL("/", href).href, selection);
}

/** Resolve just the addressed virtual child; do not expand the whole component graph. */
export function selectionNode(
  nodes: DesignNode[],
  id: string,
): DesignNode | undefined {
  const { byId } = nodeIndex(nodes);
  if (byId.has(id)) return byId.get(id);
  const [first, ...parts] = id.split("/");
  let instance = byId.get(first);
  let prefix = first;
  for (const part of parts) {
    if (instance?.kind !== "instance") return;
    const masterId = instance.props.componentId;
    const child = byId.get(part);
    if (!child) return;
    let ancestor = byId.get(child.parentId ?? "");
    const seen = new Set<string>();
    while (ancestor && ancestor.id !== masterId && !seen.has(ancestor.id)) {
      seen.add(ancestor.id);
      ancestor = byId.get(ancestor.parentId ?? "");
    }
    if (ancestor?.id !== masterId) return;
    const overrides = instance.props.overrides as
      Record<string, Record<string, unknown>> | undefined;
    instance = {
      ...child,
      id: `${prefix}/${child.id}`,
      parentId:
        child.parentId === masterId ? prefix : `${prefix}/${child.parentId}`,
      props: { ...child.props, ...overrides?.[child.id] },
    };
    prefix = instance.id;
  }
  return instance;
}
