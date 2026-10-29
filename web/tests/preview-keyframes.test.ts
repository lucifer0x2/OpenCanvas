import assert from "node:assert/strict";
import test from "node:test";
import {
  buildAnimation,
  computeOffsets,
  type Rest,
} from "../src/preview/keyframes.ts";
import { tokenTable } from "../src/preview/tokens.ts";
import type { AnimationDef } from "../src/preview/types.ts";

const shape: Rest = { opacity: 0.9, fill: "#111111", text: false };
const text: Rest = { opacity: 1, fill: "#222222", text: true };
const table = tokenTable(
  [
    {
      id: "brand",
      type: "color",
      value: { default: { aliasId: "base" }, themes: { dark: "#000000" } },
    },
    { id: "base", type: "color", value: "#2266cc" },
    { id: "slow", type: "number", value: 640 },
    { id: "lift", type: "number", value: -12 },
  ],
  [{ id: "surface", kind: "fill", value: { tokenId: "brand" } }],
);
const build = (def: Partial<AnimationDef>, rest: Rest = shape) =>
  buildAnimation(
    { name: "a", duration: 300, keyframes: [], ...def },
    rest,
    table,
  );

test("fillMode defaults to both, easing is per segment", () => {
  const { animation } = build({ keyframes: [{ opacity: 0.5 }] });
  assert.equal(animation!.timing.fill, "both");
  assert.equal(animation!.timing.easing, "linear");
  assert.equal(animation!.timing.iterations, 1);
  assert.equal(animation!.motion[0].easing, "ease");
  assert.equal(
    build({ fillMode: "none", keyframes: [{ opacity: 0 }] }).animation!.timing
      .fill,
    "none",
  );
  assert.equal(
    build({ iterations: "infinite", keyframes: [{ opacity: 0 }] }).animation!
      .timing.iterations,
    Infinity,
  );
});

test("a lone keyframe without offset is the end; rest is the start", () => {
  const { animation } = build({ keyframes: [{ opacity: 0.2 }] });
  assert.deepEqual(animation!.motion, [
    { offset: 0, easing: "ease", opacity: 0.9 },
    { offset: 1, easing: "ease", opacity: 0.2 },
  ]);
  assert.deepEqual(animation!.paint, []);
});

test("a lone offset:0 keyframe is the start; rest is the end", () => {
  const { animation } = build({
    keyframes: [{ offset: 0, opacity: 0, translateY: 24, easing: "ease-out" }],
  });
  assert.deepEqual(animation!.motion, [
    { offset: 0, easing: "ease-out", opacity: 0, translate: "0px 24px" },
    { offset: 1, easing: "ease", opacity: 0.9, translate: "0px 0px" },
  ]);
});

test("omitted offsets spread like WAAPI", () => {
  assert.deepEqual(
    computeOffsets([undefined, undefined, undefined]),
    [0, 0.5, 1],
  );
  assert.deepEqual(
    computeOffsets([undefined, undefined, 0.6, undefined]),
    [0, 0.3, 0.6, 1],
  );
  assert.deepEqual(computeOffsets([undefined]), [1]);
  const { animation } = build({
    keyframes: [{ scale: 0.5 }, { scale: 1.2 }, { scale: 1 }],
  });
  assert.deepEqual(
    animation!.motion.map((k) => [k.offset, k.scale]),
    [
      [0, "0.5"],
      [0.5, "1.2"],
      [1, "1"],
    ],
  );
});

test("keyframes inside (0, 1) get resting endpoints", () => {
  const { animation } = build({
    easing: "linear",
    keyframes: [
      { offset: 0.25, opacity: 0.5, easing: "ease-in" },
      { offset: 0.75, rotate: 90 },
    ],
  });
  assert.deepEqual(animation!.motion, [
    { offset: 0, easing: "linear", opacity: 0.9, rotate: "0deg" },
    { offset: 0.25, easing: "ease-in", opacity: 0.5 },
    { offset: 0.75, easing: "linear", rotate: "90deg" },
    { offset: 1, easing: "linear", opacity: 0.9, rotate: "0deg" },
  ]);
});

test("translate, scale and rotate land in one keyframe as CSS transform properties", () => {
  const { animation } = build({
    keyframes: [{ translateX: 10, translateY: -4, scale: 1.5, rotate: 45 }],
  });
  assert.deepEqual(animation!.motion[1], {
    offset: 1,
    easing: "ease",
    translate: "10px -4px",
    scale: "1.5",
    rotate: "45deg",
  });
  assert.deepEqual(animation!.motion[0], {
    offset: 0,
    easing: "ease",
    translate: "0px 0px",
    scale: "1",
    rotate: "0deg",
  });
});

test("fill animates the background of shapes and the color of text", () => {
  const onShape = build({ keyframes: [{ fill: "#ff0000" }] }).animation!;
  assert.deepEqual(onShape.paint, [
    { offset: 0, easing: "ease", backgroundColor: "#111111" },
    { offset: 1, easing: "ease", backgroundColor: "#ff0000" },
  ]);
  assert.deepEqual(onShape.motion, []);
  const onText = build({ keyframes: [{ fill: "#ff0000" }] }, text).animation!;
  assert.deepEqual(
    onText.paint.map((k) => k.color),
    ["#222222", "#ff0000"],
  );
});

test("tokens and fill styles resolve at the default theme", () => {
  const { animation, warnings } = build({
    duration: { tokenId: "slow" },
    keyframes: [
      { translateY: { tokenId: "lift" }, fill: { styleId: "surface" } },
    ],
  });
  assert.deepEqual(warnings, []);
  assert.equal(animation!.timing.duration, 640);
  assert.equal(animation!.motion[1].translate, "0px -12px");
  assert.equal(animation!.paint[1].backgroundColor, "#2266cc");
});

test("an unresolved reference skips that property with a warning", () => {
  const { animation, warnings } = build({
    keyframes: [{ opacity: 0.3, fill: { tokenId: "missing" } }],
  });
  assert.equal(warnings.length, 1);
  assert.match(
    warnings[0],
    /a\.keyframes\[0\]\.fill skipped: token "missing" does not exist/,
  );
  assert.equal(animation!.paint.length, 0);
  assert.equal(animation!.motion[1].opacity, 0.3);
  const typed = build({ keyframes: [{ opacity: { tokenId: "brand" } }] });
  assert.equal(typed.animation, undefined);
  assert.match(typed.warnings.join(" "), /brand" is color, not number/);
  const timing = build({
    duration: { tokenId: "nope" },
    keyframes: [{ opacity: 0 }],
  });
  assert.equal(timing.animation, undefined);
  assert.match(timing.warnings[0], /duration/);
});

test("hover is a one-way transition; click and load keep their triggers", () => {
  const hover = build({ on: "hover", keyframes: [{ scale: 1.1 }] }).animation!;
  assert.equal(hover.trigger, "hover");
  assert.deepEqual(
    [hover.timing.iterations, hover.timing.direction, hover.timing.fill],
    [1, "normal", "both"],
  );
  assert.equal(build({ keyframes: [{ scale: 2 }] }).animation!.trigger, "load");
  const click = build({
    on: "click",
    delay: 50,
    direction: "alternate",
    iterations: 2,
    keyframes: [{ scale: 2 }],
  });
  assert.deepEqual(
    [
      click.animation!.trigger,
      click.animation!.timing.delay,
      click.animation!.timing.direction,
      click.animation!.timing.iterations,
    ],
    ["click", 50, "alternate", 2],
  );
});
