import type { Snapshot } from "../types.ts";

/** A typed reference or literal, as authored in `props.animations`. */
export type Ref = number | string | { tokenId: string } | { styleId: string };

/** One authored keyframe of `props.animations[].keyframes` (see animation_catalog.rs). */
export interface KeyframeDef {
  offset?: number;
  easing?: string;
  opacity?: Ref;
  translateX?: Ref;
  translateY?: Ref;
  scale?: Ref;
  rotate?: Ref;
  fill?: Ref;
}

export type Trigger = "load" | "hover" | "click";

/** One authored entry of `props.animations`. */
export interface AnimationDef {
  name: string;
  on?: Trigger;
  duration: Ref;
  delay?: Ref;
  easing?: string;
  iterations?: number | "infinite";
  direction?: PlaybackDirection;
  fillMode?: FillMode;
  keyframes: KeyframeDef[];
}

/** One authored entry of `props.links`. */
export interface LinkDef {
  to?: string;
  on?: "click" | "hover";
  action?: "navigate" | "overlay" | "back" | "close";
}

export interface TokenDef {
  id: string;
  type: string;
  value: unknown;
}

export interface StyleDef {
  id: string;
  kind: string;
  value: unknown;
}

/** The snapshot fields the preview reads beyond the viewer's `Snapshot`. */
export interface PreviewSnapshot extends Snapshot {
  tokens: TokenDef[];
  styles: StyleDef[];
}

/** The interaction states a node may declare (INTERACTION-STATES.md). */
export const STATES = ["hover", "pressed", "focus", "disabled"] as const;
export type StateName = (typeof STATES)[number];
