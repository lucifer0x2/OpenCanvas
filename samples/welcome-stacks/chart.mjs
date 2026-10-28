export function chart(b, parent) {
  const { stack, gap, old, text, inset, source } = b;
  const inner = inset("weekly-chart", parent, "fill", 238, 22, 18, {
    minWidth: 220, maxWidth: 498, fill: { styleId: "surface-fill" }, radius: 16,
  });
  stack("chart-heading", inner, "row", "fill", 24);
  stack("chart-title-column", "chart-heading", "column", "fill", 24);
  text("chart-title", "chart-title-column", { maxWidth: 300 });
  stack("chart-dot-column", "chart-heading", "column", 8, 24);
  gap("chart-dot-column", 8);
  old("chart-legend-dot", "chart-dot-column");
  gap("chart-heading", 8, "row");
  stack("chart-legend-column", "chart-heading", "column", 82, 24);
  gap("chart-legend-column", 3);
  text("chart-legend", "chart-legend-column");
  gap("chart-heading", 7, "row");
  gap(inner, 5);
  text("chart-subtitle", inner, { maxWidth: 330 });
  gap(inner, 25);
  stack("chart-plot-row", inner, "row", "fill", 132);
  gap("chart-plot-row", 2, "row");
  // Only this visualization uses free geometry: grid lines overlap bars and
  // axis labels in a fixed local plot coordinate system, not document layout.
  stack("chart-plot", "chart-plot-row", "free", "fill", 132, { maxWidth: 450 });
  for (const [id, original] of source) {
    if (original.parentId !== "weekly-chart" || !/^(chart-grid-|day-)/.test(id)) continue;
    old(id, "chart-plot", { x: original.props.x - 24, y: original.props.y - 88 });
  }
}
