/**
 * Plays built animations on the scene DOM: `load` on mount (and Replay), `hover` forward on
 * pointer enter and back from where it is on leave, `click` restarted on every click. Links
 * are reported to the caller. With reduced motion every trigger jumps to its end state
 * (an infinite animation stays at rest).
 */
import type { Scene } from "../../../renderer/types";
import { buildAnimation, type BuiltAnimation } from "./keyframes";
import type { NodeMotion } from "./model";
import type { SceneDom, NodeElements } from "./sceneDom";
import type { TokenTable } from "./tokens";
import type { LinkDef } from "./types";

export interface MotionOptions {
  reducedMotion: boolean;
  onLink: (link: LinkDef, nodeId: string) => void;
}

export interface MotionController {
  replay: () => void;
  dispose: () => void;
  warnings: string[];
}

class Track {
  private animations: Animation[] = [];
  constructor(
    readonly id: string,
    private built: BuiltAnimation,
    private elements: NodeElements,
    private reduced: boolean,
  ) {}

  private ensure() {
    if (this.animations.length) return;
    const { motion, paint, timing } = this.built;
    if (motion.length)
      this.animations.push(
        this.elements.node.animate(motion, { ...timing, id: this.id }),
      );
    if (paint.length)
      this.animations.push(
        this.elements.box.animate(paint, { ...timing, id: `${this.id}:fill` }),
      );
  }

  private settle() {
    for (const animation of this.animations)
      if (this.built.timing.iterations === Infinity) animation.cancel();
      else animation.finish();
  }

  /** From the start, as on load or click. */
  restart() {
    this.ensure();
    for (const animation of this.animations) {
      animation.cancel();
      animation.playbackRate = 1;
      animation.play();
    }
    if (this.reduced) this.settle();
  }

  /** Towards the end (rate 1) or back to the start (rate -1) from the current time. */
  direct(rate: 1 | -1) {
    if (rate === -1 && !this.animations.length) return;
    this.ensure();
    for (const animation of this.animations) {
      if (this.reduced) {
        if (rate === 1) animation.finish();
        else animation.cancel();
        continue;
      }
      const time = animation.currentTime;
      if (
        rate === -1 &&
        (time === null || (typeof time === "number" && time <= 0))
      ) {
        // Playing backwards at the start makes WAAPI jump to the end.
        animation.cancel();
        continue;
      }
      animation.playbackRate = rate;
      animation.play();
    }
  }

  reset() {
    this.animations.forEach((animation) => animation.cancel());
    this.animations = [];
  }
}

export function wireMotion(
  dom: SceneDom,
  scene: Scene,
  motion: Map<string, NodeMotion>,
  table: TokenTable,
  options: MotionOptions,
): MotionController {
  const warnings: string[] = [];
  const tracks: Track[] = [];
  const loads: Track[] = [];
  const hovers = new Map<HTMLElement, { track: Track; explicit: boolean }[]>();
  const cleanups: (() => void)[] = [];
  const listen = <K extends keyof HTMLElementEventMap>(
    target: HTMLElement,
    type: K,
    handler: (event: HTMLElementEventMap[K]) => void,
  ) => {
    target.addEventListener(type, handler);
    cleanups.push(() => target.removeEventListener(type, handler));
  };
  for (const node of scene.nodes) {
    const declared = motion.get(node.id);
    const elements = dom.elements.get(node.id);
    if (!declared || !elements) continue;
    const rest = {
      opacity: node.opacity ?? 1,
      fill: typeof node.fill === "string" ? node.fill : "transparent",
      text: node.kind === "text",
      gradient: typeof node.fill === "object" && node.fill !== null,
    };
    const clicks: Track[] = [];
    for (const def of declared.animations) {
      const id = `${node.id}:${def.name}`;
      const result = buildAnimation(
        def,
        rest,
        {
          ...table,
          theme: node.effectiveTheme ?? scene.provenance.theme,
        },
        id,
      );
      warnings.push(...result.warnings);
      if (!result.animation) continue;
      const trigger = result.animation.trigger;
      const explicit = def.triggerNodeId !== undefined;
      const source =
        trigger === "hover"
          ? dom.elements.get(def.triggerNodeId ?? node.id)?.node
          : undefined;
      if (trigger === "hover" && !source) {
        warnings.push(
          `${id}: hover trigger "${def.triggerNodeId}" is missing from the preview; animation skipped`,
        );
        continue;
      }
      const track = new Track(
        id,
        result.animation,
        elements,
        options.reducedMotion,
      );
      tracks.push(track);
      if (trigger === "load") loads.push(track);
      if (trigger === "click") clicks.push(track);
      if (source) {
        const bindings = hovers.get(source) ?? [];
        bindings.push({ track, explicit });
        hovers.set(source, bindings);
      }
    }
    const clickLink = declared.links.find(
      (link) => (link.on ?? "click") === "click",
    );
    for (const link of declared.links)
      if (link.on === "hover")
        warnings.push(
          `${node.id}: hover ${link.action ?? "navigate"} link is not supported in preview`,
        );
    if (clicks.length || clickLink) {
      elements.node.classList.add("oc-clickable");
      listen(elements.node, "click", (event) => {
        clicks.forEach((track) => track.restart());
        if (!clickLink) return;
        event.stopPropagation();
        options.onLink(clickLink, node.id);
      });
    }
  }
  for (const [source, bindings] of hovers) {
    const direct = (event: PointerEvent, rate: 1 | -1) => {
      for (const { track, explicit } of bindings)
        if (!explicit || event.pointerType !== "touch") track.direct(rate);
    };
    listen(source, "pointerenter", (event) => direct(event, 1));
    listen(source, "pointerleave", (event) => direct(event, -1));
  }
  const replay = () => {
    tracks.forEach((track) => track.reset());
    loads.forEach((track) => track.restart());
  };
  replay();
  return {
    replay,
    warnings,
    dispose: () => {
      cleanups.forEach((cleanup) => cleanup());
      tracks.forEach((track) => track.reset());
    },
  };
}
