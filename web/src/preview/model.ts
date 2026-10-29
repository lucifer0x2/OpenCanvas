/** Pure reads of the authored snapshot for the preview: frames, declared states, motion. */
import { children } from "../state.ts";
import type { DesignNode } from "../types.ts";
import {
  STATES,
  type AnimationDef,
  type LinkDef,
  type StateName,
} from "./types.ts";

/** Root frames of every page, in page then sibling order. */
export function rootFrames(nodes: DesignNode[]): DesignNode[] {
  return children(nodes, null)
    .filter((node) => node.kind === "page")
    .flatMap((page) =>
      children(nodes, page.id).filter((node) => node.kind === "frame"),
    );
}

/** Authored frame size with the viewer's explicit 800×600 fallback for nonnumeric values. */
export function frameSize(frame: DesignNode) {
  const dimension = (value: unknown, fallback: number) =>
    typeof value === "number" && value > 0 ? value : fallback;
  return {
    width: dimension(frame.props.width, 800),
    height: dimension(frame.props.height, 600),
  };
}

/**
 * IDs of the frame subtree plus, transitively, the masters of its instances: expansion keeps
 * master declarations on virtual children (`<instanceId>/<masterChildId>`), so a master's
 * `states` matter to the frame although the master lives elsewhere.
 */
export function effectiveSubtree(nodes: DesignNode[], frameId: string) {
  const byParent = new Map<string | null, DesignNode[]>();
  for (const node of nodes)
    byParent.set(node.parentId, [...(byParent.get(node.parentId) ?? []), node]);
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const seen = new Set<string>();
  const queue = [frameId];
  while (queue.length) {
    const id = queue.pop()!;
    if (seen.has(id) || !byId.has(id)) continue;
    seen.add(id);
    for (const child of byParent.get(id) ?? []) queue.push(child.id);
    const master = byId.get(id)!.props.componentId;
    if (typeof master === "string") queue.push(master);
  }
  return [...seen].map((id) => byId.get(id)!);
}

/** Non-empty interaction states declared in the frame (including instance masters). */
export function declaredStates(
  nodes: DesignNode[],
  frameId: string,
): StateName[] {
  const declared = new Set<StateName>();
  for (const node of effectiveSubtree(nodes, frameId)) {
    const states = node.props.states;
    if (!states || typeof states !== "object") continue;
    for (const name of STATES) {
      const value = (states as Record<string, unknown>)[name];
      if (value && typeof value === "object" && Object.keys(value).length)
        declared.add(name);
    }
  }
  return STATES.filter((name) => declared.has(name));
}

export interface NodeMotion {
  animations: AnimationDef[];
  links: LinkDef[];
}

/** Authored animations and links by node ID (component subtrees cannot author either). */
export function nodeMotion(nodes: DesignNode[]): Map<string, NodeMotion> {
  const result = new Map<string, NodeMotion>();
  for (const node of nodes) {
    const animations = Array.isArray(node.props.animations)
      ? (node.props.animations as AnimationDef[])
      : [];
    const links = Array.isArray(node.props.links)
      ? (node.props.links as LinkDef[])
      : [];
    if (animations.length || links.length)
      result.set(node.id, { animations, links });
  }
  return result;
}
