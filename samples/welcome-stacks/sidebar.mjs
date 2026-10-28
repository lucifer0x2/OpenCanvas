export function sidebar(b, frame) {
  const { stack, gap, old, text, inset } = b;
  stack("sidebar", frame, "column", 176, "fill", {
    minWidth: 176, maxWidth: 176, fill: { styleId: "surface-fill" },
  });
  gap("sidebar", 30);
  const brand = inset("brand-row", "sidebar", "fill", 34, 24, 0);
  stack("brand-items", brand, "row", "fill", 34, { layout: { mode: "row", padding: 0, gap: 11 } });
  // A glyph drawn over the logo tile is a real decorative overlay.
  stack("logo-overlay", "brand-items", "free", 32, 34);
  old("logo", "logo-overlay", { x: 0, y: 2 });
  old("logo-letter", "logo-overlay", { x: 9, y: 0 });
  stack("brand-name-column", "brand-items", "column", "fill", 34);
  gap("brand-name-column", 3);
  text("brand-name", "brand-name-column");
  gap("sidebar", 40);
  text("workspace-label", inset("workspace-inset", "sidebar", "fill", 14, 24, 0));
  gap("sidebar", 18);
  const nav = inset("nav-inset", "sidebar", "fill", 44, 16, 0);
  stack("active-nav", nav, "row", 144, 44, { fill: "#EEEDFF", radius: 10 });
  text("overview-nav", inset("active-nav-inset", "active-nav", "fill", "fill", 16, 10), { maxWidth: 118 });
  gap("sidebar", 20);
  text("projects-nav", inset("projects-inset", "sidebar", "fill", 20, 32, 0), { maxWidth: 118 });
  gap("sidebar", 26);
  text("calendar-nav", inset("calendar-inset", "sidebar", "fill", 20, 32, 0), { maxWidth: 125 });
  // Fill spacer anchors the footer to the bottom as viewport height changes.
  stack("sidebar-flex-space", "sidebar", "column", "fill", "fill", { minHeight: 0 });
  const footer = inset("sidebar-footer", "sidebar", "fill", 95, 24, 0);
  old("sidebar-divider", footer);
  gap(footer, 20);
  text("profile-name", footer, { maxWidth: 144 });
  gap(footer, 6);
  text("profile-plan", footer, { maxWidth: 145 });
}
