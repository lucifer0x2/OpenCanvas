// Scenes for opacity.test.mjs: each pairs scene nodes with the equivalent CSS div tree.
import { node } from "./fixtures.mjs";
import { div } from "./css-oracle.mjs";

export const frame = () =>
  node("frame", null, "frame", { fill: "#ffffff", layout: { mode: "free" } });
export const box = (id, parent, x, y, width, height, props = {}) =>
  node(id, parent, "rectangle", { x, y, width, height, ...props });
export const group = (id, parent, x, y, width, height, props) =>
  node(id, parent, "group", {
    x,
    y,
    width,
    height,
    layout: { mode: "free" },
    ...props,
  });

export const overlap = () => [
  frame(),
  group("g", "frame", 10, 10, 120, 60, { opacity: 0.5 }),
  box("red", "g", 0, 0, 60, 60, { fill: "#ef4444" }),
  box("blue", "g", 40, 0, 60, 60, { fill: "#2563eb" }),
];
export const overlapCSS = div(
  10,
  10,
  120,
  60,
  "opacity:.5",
  div(0, 0, 60, 60, "background:#ef4444") +
    div(40, 0, 60, 60, "background:#2563eb"),
);
export const nested = () => [
  frame(),
  group("A", "frame", 10, 10, 160, 80, { opacity: 0.5 }),
  box("green", "A", 0, 0, 160, 30, { fill: "#16a34a" }),
  group("B", "A", 20, 10, 120, 60, { opacity: 0.5 }),
  box("red", "B", 0, 0, 60, 60, { fill: "#ef4444" }),
  box("blue", "B", 40, 0, 60, 60, { fill: "#2563eb" }),
];
export const nestedCSS = div(
  10,
  10,
  160,
  80,
  "opacity:.5",
  div(0, 0, 160, 30, "background:#16a34a") +
    div(
      20,
      10,
      120,
      60,
      "opacity:.5",
      div(0, 0, 60, 60, "background:#ef4444") +
        div(40, 0, 60, 60, "background:#2563eb"),
    ),
);
export const clipped = () => [
  frame(),
  group("g", "frame", 20, 20, 80, 80, {
    opacity: 0.6,
    clip: true,
    radius: 20,
    fill: "#e2e8f0",
  }),
  box("big", "g", -20, -20, 120, 60, { fill: "#ef4444" }),
  box("over", "g", 30, 30, 80, 80, { fill: "#2563eb" }),
];
export const clippedCSS = div(
  20,
  20,
  80,
  80,
  "opacity:.6;overflow:hidden;border-radius:20px;background:#e2e8f0",
  div(-20, -20, 120, 60, "background:#ef4444") +
    div(30, 30, 80, 80, "background:#2563eb"),
);

/** The note #10 keycap: shadow, bottom border, border and background under a label, at 0.8. */
export const keycap = () => [
  frame(),
  group("key", "frame", 20, 20, 40, 24, { opacity: 0.8 }),
  box("shadow", "key", 0, 1, 40, 24, { radius: 5, fill: "#00000033" }),
  box("bottom", "key", 0, 0, 40, 23, { radius: 5, fill: "#94a3b8" }),
  box("border", "key", 0, 0, 40, 22, { radius: 5, fill: "#cbd5e1" }),
  box("bg", "key", 1, 1, 38, 20, { radius: 4, fill: "#f8fafc" }),
  node("label", "key", "text", {
    x: 8,
    y: 2,
    text: "⌘K",
    fontId: "font_bundled",
    fontSize: 12,
    lineHeight: 18,
    fill: "#334155",
  }),
];

/** [nodes, CSS, points inside flat regions] for the CSS oracle comparison. */
export const cssCases = () => [
  [
    overlap(),
    overlapCSS,
    [
      [30, 40],
      [60, 40],
      [90, 40],
    ],
  ],
  [
    nested(),
    nestedCSS,
    [
      [15, 15],
      [50, 30],
      [80, 30],
      [80, 60],
      [110, 60],
    ],
  ],
  [
    clipped(),
    clippedCSS,
    [
      [21, 21],
      [50, 30],
      [70, 50],
      [95, 95],
      [110, 60],
      [60, 90],
    ],
  ],
];
