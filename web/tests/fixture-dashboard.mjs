// The 1440x900 dashboard behind the "dense" and "stress" zoom fixtures (fixture-dense.mjs).

export const node = (id, kind, bounds, fill, extras = {}) => ({
  id,
  kind,
  parentId: id === "frame" ? null : "frame",
  bounds,
  fill,
  lines: [],
  fontId: null,
  fontSize: 13,
  lineHeight: 18,
  assetId: null,
  fit: "cover",
  opacity: 1,
  clip: false,
  radius: 0,
  crop: null,
  ...extras,
});

const text = (id, x, y, size, value, fill = "#26364a") => {
  const lineHeight = Math.round(size * 1.4);
  const width = Math.ceil(value.length * size * 0.55);
  return node(id, "text", { x, y, width, height: lineHeight }, fill, {
    fontId: "font_bundled",
    fontSize: size,
    lineHeight,
    lines: [{ text: value, x: 0, y: Math.round(lineHeight * 0.74), width }],
  });
};

/** One 1440x900 dashboard at (ox, oy); `p` prefixes IDs so tiles stay unique. */
export function dashboard(p, ox, oy) {
  const out = [
    node(
      `${p}side`,
      "rectangle",
      { x: ox, y: oy, width: 240, height: 900 },
      "#f4f6f8",
    ),
    text(`${p}logo`, ox + 24, oy + 22, 20, "OpenCanvas Metrics"),
  ];
  const nav = [
    "Overview",
    "Revenue",
    "Customers",
    "Orders",
    "Products",
    "Inventory",
    "Reports",
    "Campaigns",
    "Messages",
    "Calendar",
    "Team",
    "Billing",
    "Settings",
    "Help center",
  ];
  nav.forEach((label, i) => {
    const y = oy + 84 + i * 40;
    out.push(
      node(
        `${p}nav-icon-${i}`,
        "rectangle",
        { x: ox + 24, y: y + 1, width: 16, height: 16 },
        "#94a3b8",
        { radius: 4 },
      ),
    );
    out.push(text(`${p}nav-${i}`, ox + 52, y, 13, label, "#334155"));
  });
  out.push(
    node(
      `${p}top`,
      "rectangle",
      { x: ox + 240, y: oy, width: 1200, height: 64 },
      "#ffffff",
    ),
    text(
      `${p}crumb`,
      ox + 272,
      oy + 20,
      16,
      "Workspace / Analytics / Quarterly overview",
    ),
    node(
      `${p}search`,
      "rectangle",
      { x: ox + 1128, y: oy + 14, width: 280, height: 36 },
      "#eef2f6",
      { radius: 8 },
    ),
    text(
      `${p}search-text`,
      ox + 1144,
      oy + 23,
      13,
      "Search reports and people",
      "#64748b",
    ),
  );
  ["Revenue", "Active users", "Conversion", "Churn"].forEach((label, i) => {
    const x = ox + 272 + i * 290,
      y = oy + 92;
    out.push(
      node(
        `${p}kpi-${i}`,
        "rectangle",
        { x, y, width: 268, height: 128 },
        "#f8fafc",
        { radius: 12 },
      ),
      text(`${p}kpi-label-${i}`, x + 20, y + 18, 13, label, "#64748b"),
      text(
        `${p}kpi-value-${i}`,
        x + 20,
        y + 42,
        32,
        ["$482,910", "12,408", "3.84%", "1.2%"][i],
      ),
      text(
        `${p}kpi-delta-${i}`,
        x + 20,
        y + 94,
        12,
        `+${(i + 2) * 1.7}% vs last quarter`,
        "#15803d",
      ),
    );
  });
  out.push(
    node(
      `${p}table`,
      "rectangle",
      { x: ox + 272, y: oy + 248, width: 1136, height: 636 },
      "#ffffff",
      { radius: 12 },
    ),
    node(
      `${p}photo`,
      "image",
      { x: ox + 24, y: oy + 660, width: 192, height: 128 },
      "transparent",
      { assetId: "stripes", radius: 8 },
    ),
  );
  const columns = ["Account", "Owner", "Region", "Plan", "Seats", "Renewal"];
  for (let row = 0; row < 25; row++) {
    const y = oy + 256 + row * 25;
    if (row % 2 === 1)
      out.push(
        node(
          `${p}row-${row}`,
          "rectangle",
          { x: ox + 280, y, width: 1120, height: 25 },
          "#f1f5f9",
        ),
      );
    columns.forEach((name, col) => {
      const value =
        row === 0 ? name.toUpperCase() : `${name} ${row}-${col * 7 + row}`;
      out.push(
        text(
          `${p}cell-${row}-${col}`,
          ox + 296 + col * 184,
          y + 3,
          row === 0 ? 12 : 13,
          value,
        ),
      );
    });
  }
  return out;
}
