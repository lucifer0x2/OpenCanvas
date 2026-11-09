import { node, scene, stroke, shadow } from "./scene.mjs";
export function fixtures(resources) {
  const cases = [], rect = props => [node("shape", 28, 26, 180, 102, props)];
  const add = (name, nodes, category = "nontext", width, height) =>
    cases.push({ name, category, scales: [1, 2], scene: scene(name, nodes, resources, width, height) });
  add("solid-alpha", rect({ fill: "#ee332277" }));
  add("solid-opaque", rect({ fill: "#369ac5" }));
  add("scale-fractional-ceil", [node("shape", 0, 0, 40, 40)], "nontext", 40, 40);
  cases.at(-1).scales = [1, 2, 2.4];
  add("linear-oblique", rect({ fill: { type: "linear", angle: 123, stops: [[0, "#ef4444"], [1, "#2563eb"]] } }));
  add("gradient-hard-stops", rect({ fill: { type: "linear", angle: 90, stops: [[0, "#ff0000"], [0.5, "#ff0000"], [0.5, "#0000ff"], [1, "#0000ff"]] } }));
  add("gradient-alpha", rect({ fill: { type: "linear", angle: 73, stops: [[0, "#ff000000"], [0.4, "#00ff0044"], [1, "#0000ffff"]] } }));
  add("radial-elliptical", rect({ fill: { type: "radial", center: [0.35, 0.6], radius: [125, 58], stops: [[0, "#ffff00ff"], [0.55, "#00aaff99"], [1, "#00000000"]] } }));
  add("per-corner-radius", rect({ radius: [3, 36, 72, 16] }));
  for (const align of ["inside", "center", "outside"]) {
    add(`stroke-${align}-sides`, rect({ radius: [8, 30, 18, 45], stroke: stroke(align) }));
    add(`stroke-${align}-uniform`, rect({ radius: 18, stroke: { align, width: [7, 7, 7, 7], color: Array(4).fill("#231a68aa") } }));
  }
  add("nested-alpha-clip-stroke-after-children", [
    node("outer", 25, 20, 185, 120, { kind: "frame", opacity: 0.65, clip: true, radius: [10, 32, 22, 6], fill: "#ffe9b7", stroke: stroke("outside") }),
    node("inner", 40, 35, 155, 90, { parentId: "outer", kind: "group", opacity: 0.55, clip: true, radius: 20, fill: "#2277aacc" }),
    node("overflow", 0, 60, 240, 45, { parentId: "inner", fill: "#ff2255" }),
    node("above", 150, 4, 65, 150, { parentId: "outer", fill: "#0066cc99" }),
  ]);
  add("group-alpha-overlap", [node("group", 20, 20, 190, 115, { kind: "group", opacity: 0.4, fill: "transparent" }),
    node("a", 25, 25, 130, 95, { parentId: "group", fill: "#ee3355" }),
    node("b", 95, 45, 110, 90, { parentId: "group", fill: "#2255dd" })]);
  for (const inset of [false, true]) for (const spread of [-5, 0, 6]) {
    add(`shadow-${inset ? "inset" : "outer"}-spread-${spread}`, rect({ radius: [8, 24, 16, 32], shadow: [shadow(inset, spread)] }));
  }
  add("shadow-order-multiple", rect({ fill: "#eeeeee", radius: 18,
    shadow: [shadow(false, 3, 5), { ...shadow(false, -1, 3), x: -7, color: "#ed225599" }, shadow(true, 2, 7)] }));
  for (const asset of resources.assets) for (const fit of ["contain", "cover", "fit"]) {
    add(`image-${asset.id}-${fit}`, rect({ kind: "image", assetId: asset.id, fill: "transparent", fit, radius: [2, 18, 30, 8] }));
  }
  add("image-normalized-crop", rect({ kind: "image", assetId: "png", fill: "#bbccdd", fit: "cover", crop: { x: 0.2, y: 0.1, width: 0.55, height: 0.7 }, radius: 15 }));
  const text = (id, y, fontId, more = {}) => node(id, 18, y, 205, 65, { kind: "text", fontId, fill: "#183860", fontSize: 27, lineHeight: 34,
    lines: [{ text: "Ấy Việt fj AV", x: -2, y: 31, width: 176 }, { text: "saved line list", x: 6, y: 60, width: 183 }], ...more });
  add("text-saved-lines-tracking-decoration", [text("text", 14, "noto", { letterSpacing: 1.25, decoration: { line: "underline", offset: 3, thickness: 2 } })], "text");
  add("text-overlap-alpha", [text("overlap", 15, "noto", { opacity: 0.45,
    lines: [{ text: "OVERLAP", x: 0, y: 48, width: 178 }, { text: "OVERLAP", x: 11, y: 48, width: 178 }],
    decoration: { line: "line-through", offset: -8, thickness: 3 } })], "text");
  add("text-static-faces", [text("regular", 4, "lato-regular", { lines: [{ text: "Lato Regular Ấy", x: 0, y: 30, width: 190 }] }),
    text("bold", 49, "lato-bold", { lines: [{ text: "Lato Bold AV", x: 0, y: 30, width: 180 }] }),
    text("italic", 94, "lato-italic", { lines: [{ text: "Lato Italic fj", x: -2, y: 30, width: 180 }] })], "text");
  add("text-inter-mixed-greek-latin-combining-dot", [text("mixed", 10, "inter", { fontSize: 32,
    lines: [{ text: "ς i\u0307", x: 0, y: 35, width: 46.796875 }, { text: "aς i\u0307 Z α", x: 0, y: 82, width: 150 }] })], "text");
  // Exact G2 current/inter/86 (lowercase/23), original source ΟΣ ΟΣΑ Σ ΣΟΣ İ ẞ.
  // These saved output lines are not transformed/wrapped a second time by paint.
  add("text-inter-lowercase-expansion", [node("lowercase", 28, 12, 23, 161, { kind: "text", fontId: "inter",
    fill: "#183860", fontSize: 16, lineHeight: 23,
    lines: ["ος", "οσ", "α", "σ", "σο", "ς i\u0307", "ß"].map((text, i) => ({ text, x: 0, y: 17 + i * 23,
      width: [18.53125, 19.4375, 10.5625, 9.921875, 19.171875, 17.296875, 9.796875][i] })),
  })], "text", 240, 200);
  cases.at(-1).scales = [1, 2, 3];
  // Full-scene atlases supplement, never replace isolated feature gates.
  for (const category of ["nontext", "text"]) {
    const selected = cases.filter(c => c.category === category), nodes = [];
    const tileHeight = category === "text" ? 208 : 160;
    selected.forEach((c, i) => {
      const dx = i % 4 * 240, dy = Math.floor(i / 4) * tileHeight;
      nodes.push(...c.scene.nodes.map(n => ({ ...n, id: `${i}-${n.id}`, parentId: n.parentId === null ? null : `${i}-${n.parentId}`,
        bounds: { ...n.bounds, x: n.bounds.x + dx, y: n.bounds.y + dy } })));
    });
    add(`full-${category}-atlas`, nodes, category, 960, Math.ceil(selected.length / 4) * tileHeight);
  }
  return cases;
}
