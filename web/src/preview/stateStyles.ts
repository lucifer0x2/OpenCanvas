/**
 * Interaction-state visuals as a stylesheet. Each declared state's scene (theme/token resolved
 * by the server, same layout) is diffed per node against the default scene; the differences
 * become `!important` rules on the state's selector, so they win over the resting inline
 * styles and over animation fills of the same property. Pure.
 */
import type { Scene, SceneNode } from "../../../renderer/types.ts";
import {
  boxDeclarations,
  changed,
  nodeDeclarations,
  strokeDeclarations,
  type Declarations,
} from "./paintStyle.ts";
import type { StateName } from "./types.ts";

/** The CSS condition of each state (INTERACTION-STATES.md); later entries win ties. */
export const STATE_SELECTORS: [StateName, string][] = [
  ["hover", ":hover"],
  ["focus", ":focus-visible"],
  ["pressed", ":active"],
  ["disabled", "[data-oc-disabled]"],
];

export interface StateStyles {
  css: string;
  /** Nodes whose stroke overlay is needed by the default scene or by some state. */
  strokeIds: Set<string>;
  /** Nodes that differ in at least one state, per state. */
  changedIds: Map<StateName, string[]>;
}

export const nodeSelector = (id: string) =>
  `[data-oc-id="${id.replace(/["\\]/g, (c) => `\\${c}`)}"]`;

const rule = (selector: string, declarations: Declarations) => {
  const body = Object.entries(declarations)
    .map(([key, value]) => `${key}:${value} !important`)
    .join(";");
  return body ? `${selector}{${body}}` : "";
};

function nodeRules(selector: string, base: SceneNode, state: SceneNode) {
  const stroke = changed(
    strokeDeclarations(base.stroke, base.radius ?? 0),
    strokeDeclarations(state.stroke, state.radius ?? 0),
  );
  return [
    rule(selector, changed(nodeDeclarations(base), nodeDeclarations(state))),
    rule(
      `${selector}>.oc-box`,
      changed(boxDeclarations(base), boxDeclarations(state)),
    ),
    // A stroke replaces the whole stroke object: write every declaration once one differs.
    Object.keys(stroke).length
      ? rule(
          `${selector}>.oc-stroke`,
          strokeDeclarations(state.stroke, state.radius ?? 0),
        )
      : "",
  ].filter(Boolean);
}

export function stateStyles(
  base: Scene,
  states: Partial<Record<StateName, Scene>>,
): StateStyles {
  const strokeIds = new Set(
    base.nodes.filter((n) => n.stroke).map((n) => n.id),
  );
  const byId = new Map(base.nodes.map((node) => [node.id, node]));
  const css: string[] = [];
  const changedIds = new Map<StateName, string[]>();
  for (const [name, condition] of STATE_SELECTORS) {
    const scene = states[name];
    if (!scene) continue;
    const ids: string[] = [];
    for (const node of scene.nodes) {
      const resting = byId.get(node.id);
      if (!resting) continue;
      if (node.stroke) strokeIds.add(node.id);
      const rules = nodeRules(
        `${nodeSelector(node.id)}${condition}`,
        resting,
        node,
      );
      if (rules.length) {
        ids.push(node.id);
        css.push(...rules);
      }
    }
    if (ids.length) changedIds.set(name, ids);
  }
  return { css: css.join("\n"), strokeIds, changedIds };
}
