/**
 * The motion spec of `props.animations` (crates/storage/src/animation_catalog.rs) as WAAPI
 * keyframes. Pure: no DOM access, so the rules are unit tested in Node.
 *
 * - Omitted offsets spread like WAAPI; a lone keyframe without offset is offset 1.
 * - A property missing at offset 0 or 1 takes the node's resting value, written explicitly.
 * - `fillMode` defaults to `both`; `easing` applies per segment (the effect itself is linear).
 * - translateX/Y, scale and rotate map to the CSS `translate`, `scale` and `rotate`
 *   properties (one per validator channel), so two animations on one node never clobber.
 * - `fill` targets the paint element: `backgroundColor` on shapes, `color` on text; over a
 *   gradient resting fill it is skipped with a warning (the gradient image would hide it).
 */
import { resolveColor, resolveNumber, type TokenTable } from "./tokens.ts";
import type { AnimationDef, KeyframeDef, Trigger } from "./types.ts";

/** Resting (scene, default state) values of the animated node. */
export interface Rest {
  opacity: number;
  fill: string;
  text: boolean;
  /** The resting fill is a gradient: it would paint over an animated solid fill. */
  gradient?: boolean;
}

export interface BuiltAnimation {
  name: string;
  trigger: Trigger;
  /** Keyframes for the node element: opacity, translate, scale, rotate. */
  motion: Keyframe[];
  /** Keyframes for the paint element: backgroundColor (shapes) or color (text). */
  paint: Keyframe[];
  timing: KeyframeEffectOptions;
}

export interface BuildResult {
  animation?: BuiltAnimation;
  warnings: string[];
}

const DEFAULT_EASING = "ease";

/** WAAPI computed offsets: lone keyframe = 1, else first 0 and last 1, gaps spread evenly. */
export function computeOffsets(authored: (number | undefined)[]): number[] {
  const n = authored.length;
  const offsets = [...authored];
  if (n === 1) offsets[0] ??= 1;
  else if (n > 1) {
    offsets[0] ??= 0;
    offsets[n - 1] ??= 1;
  }
  let start = 0;
  for (let k = 1; k < n; k++) {
    const end = offsets[k];
    if (end === undefined) continue;
    const from = offsets[start]!;
    for (let step = start + 1; step < k; step++)
      offsets[step] = from + ((end - from) * (step - start)) / (k - start);
    start = k;
  }
  return offsets as number[];
}

type Props = Record<string, string | number>;

function restProps(rest: Rest): { motion: Props; paint: Props } {
  return {
    motion: {
      opacity: rest.opacity,
      translate: "0px 0px",
      scale: "1",
      rotate: "0deg",
    },
    paint: { [rest.text ? "color" : "backgroundColor"]: rest.fill },
  };
}

/** One authored keyframe → CSS values per target, skipping unresolved references. */
function keyframeProps(
  frame: KeyframeDef,
  rest: Rest,
  table: TokenTable,
  at: string,
  warnings: string[],
): { motion: Props; paint: Props } {
  const motion: Props = {};
  const paint: Props = {};
  const number = (key: keyof KeyframeDef) => {
    if (frame[key] === undefined) return undefined;
    const resolved = resolveNumber(table, frame[key]);
    if ("warning" in resolved) {
      warnings.push(`${at}.${key} skipped: ${resolved.warning}`);
      return undefined;
    }
    return resolved.value;
  };
  const opacity = number("opacity");
  if (opacity !== undefined) motion.opacity = opacity;
  const x = number("translateX");
  const y = number("translateY");
  if (x !== undefined || y !== undefined)
    motion.translate = `${x ?? 0}px ${y ?? 0}px`;
  const scale = number("scale");
  if (scale !== undefined) motion.scale = String(scale);
  const rotate = number("rotate");
  if (rotate !== undefined) motion.rotate = `${rotate}deg`;
  if (frame.fill !== undefined) {
    const fill = resolveColor(table, frame.fill);
    if ("warning" in fill) warnings.push(`${at}.fill skipped: ${fill.warning}`);
    else paint[rest.text ? "color" : "backgroundColor"] = fill.value;
  }
  return { motion, paint };
}

/** Drops empty keyframes and writes the resting value of every property missing at 0 or 1. */
function withRestEndpoints(
  frames: { offset: number; easing: string; props: Props }[],
  rest: Props,
  easing: string,
): Keyframe[] {
  const used = frames.filter((frame) => Object.keys(frame.props).length);
  if (!used.length) return [];
  const result = used.map((f) => ({
    ...f.props,
    offset: f.offset,
    easing: f.easing,
  }));
  const properties = [...new Set(used.flatMap((f) => Object.keys(f.props)))];
  if (result[0].offset !== 0) result.unshift({ offset: 0, easing });
  if (result[result.length - 1].offset !== 1)
    result.push({ offset: 1, easing });
  const [first, last] = [result[0], result[result.length - 1]];
  for (const property of properties) {
    const at = (frame: Keyframe) => property in frame;
    if (!at(first)) (first as Props)[property] = rest[property];
    if (!at(last)) (last as Props)[property] = rest[property];
  }
  return result as Keyframe[];
}

export function buildAnimation(
  def: AnimationDef,
  rest: Rest,
  table: TokenTable,
  label = def.name,
): BuildResult {
  const warnings: string[] = [];
  const duration = resolveNumber(table, def.duration);
  if ("warning" in duration)
    return {
      warnings: [`${label}.duration: ${duration.warning}; animation skipped`],
    };
  let delay = 0;
  if (def.delay !== undefined) {
    const resolved = resolveNumber(table, def.delay);
    if ("warning" in resolved)
      warnings.push(`${label}.delay ignored: ${resolved.warning}`);
    else delay = resolved.value;
  }
  const trigger = def.on ?? "load";
  const easing = def.easing ?? DEFAULT_EASING;
  const authored = def.keyframes ?? [];
  const offsets = computeOffsets(authored.map((frame) => frame.offset));
  const motion: { offset: number; easing: string; props: Props }[] = [];
  const paint: typeof motion = [];
  authored.forEach((frame, j) => {
    const props = keyframeProps(
      frame,
      rest,
      table,
      `${label}.keyframes[${j}]`,
      warnings,
    );
    const segment = frame.easing ?? easing;
    motion.push({ offset: offsets[j], easing: segment, props: props.motion });
    paint.push({ offset: offsets[j], easing: segment, props: props.paint });
  });
  const restValues = restProps(rest);
  let painted = withRestEndpoints(paint, restValues.paint, easing);
  // Storage rejects this pair; older data still gets a visible warning, not a silent no-op.
  if (rest.gradient && painted.length) {
    warnings.push(
      `${label}: fill animation skipped: the resting fill is a gradient, which hides an animated solid fill (fill animation requires a solid resting fill)`,
    );
    painted = [];
  }
  const built: BuiltAnimation = {
    name: def.name,
    trigger,
    motion: withRestEndpoints(motion, restValues.motion, easing),
    paint: painted,
    timing: {
      duration: duration.value,
      delay,
      easing: "linear",
      // A hover animation is a one-way transition (the validator forbids these three keys).
      iterations:
        trigger === "hover"
          ? 1
          : def.iterations === "infinite"
            ? Infinity
            : (def.iterations ?? 1),
      direction: trigger === "hover" ? "normal" : (def.direction ?? "normal"),
      fill: trigger === "hover" ? "both" : (def.fillMode ?? "both"),
    },
  };
  if (!built.motion.length && !built.paint.length)
    return {
      warnings: [
        ...warnings,
        `${label}: no playable keyframes; animation skipped`,
      ],
    };
  return { animation: built, warnings };
}
