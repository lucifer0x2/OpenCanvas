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
  const cleanups: (() => void)[] = [];
  const listen = (
    target: HTMLElement,
    type: string,
    handler: (event: Event) => void,
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
    const byTrigger = {
      load: [] as Track[],
      hover: [] as Track[],
      click: [] as Track[],
    };
    for (const def of declared.animations) {
      const id = `${node.id}:${def.name}`;
      const result = buildAnimation(def, rest, table, id);
      warnings.push(...result.warnings);
      if (!result.animation) continue;
      const track = new Track(
        id,
        result.animation,
        elements,
        options.reducedMotion,
      );
      tracks.push(track);
      byTrigger[result.animation.trigger].push(track);
    }
    loads.push(...byTrigger.load);
    if (byTrigger.hover.length) {
      listen(elements.node, "pointerenter", () =>
        byTrigger.hover.forEach((t) => t.direct(1)),
      );
      listen(elements.node, "pointerleave", () =>
        byTrigger.hover.forEach((t) => t.direct(-1)),
      );
    }
    const clickLink = declared.links.find(
      (link) => (link.on ?? "click") === "click",
    );
    for (const link of declared.links)
      if (link.on === "hover")
        warnings.push(
          `${node.id}: hover ${link.action ?? "navigate"} link is not supported in preview`,
        );
    if (byTrigger.click.length || clickLink) {
      elements.node.classList.add("oc-clickable");
      listen(elements.node, "click", (event) => {
        byTrigger.click.forEach((track) => track.restart());
        if (!clickLink) return;
        event.stopPropagation();
        options.onLink(clickLink, node.id);
      });
    }
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
