import { builder, DESKTOP, FRAME } from "./helpers.mjs";
import { sidebar } from "./sidebar.mjs";
import { header, stats, tasks } from "./sections.mjs";
import { chart } from "./chart.mjs";

export { FRAME, DESKTOP, NARROW, THRESHOLD } from "./helpers.mjs";

export function stacksOperations() {
  const b = builder();
  const { node, stack, gap, old, text, inset } = b;
  node("welcome-page-stacks", "page", null, { name: "Mẫu stacks" });
  node(FRAME, "frame", "welcome-page-stacks", {
    name: "Một ngày thật tốt · Stacks", ...DESKTOP, clip: true,
    fill: { tokenId: "canvas" }, layout: { mode: "row", padding: 0, gap: 0 },
  });
  sidebar(b, FRAME);
  stack("body", FRAME, "row", "fill", "fill", { minWidth: 480, maxWidth: 2000 });
  gap("body", 32, "row");
  stack("body-content", "body", "column", "fill", "fill");
  gap("body", 48, "row");
  gap("body-content", 28);
  header(b, "body-content");
  gap("body-content", 20);
  stats(b, "body-content");
  gap("body-content", 20);
  stack("weekly-row", "body-content", "row", "fill", 238, { layout: { mode: "row", padding: 0, gap: 20 } });
  chart(b, "weekly-row");
  const goal = inset("weekly-goal", "weekly-row", "fill", 238, 22, 23, {
    minWidth: 160, maxWidth: 266, fill: "#EEEDFF", radius: 16,
  });
  text("goal-label", goal, { maxWidth: 230 });
  gap(goal, 12);
  text("goal-value", goal, { maxWidth: 230 });
  gap(goal, 10);
  text("goal-caption", goal, { maxWidth: 230 });
  gap(goal, 26);
  // The group paints the track behind its ordinary flow child. A maxWidth cap
  // preserves the original desktop progress; percentage progress is absent in v1.
  stack("goal-track", goal, "row", "fill", 8, { maxWidth: 222, fill: "#DCD9FB", radius: 4 });
  old("goal-progress", "goal-track", { width: "fill", maxWidth: 178 });
  gap(goal, 18);
  text("goal-details", goal, { maxWidth: 230 });
  gap("body-content", 20);
  tasks(b, "body-content");
  return b.operations;
}
