export function header(b, parent) {
  const { stack, gap, text, old, inset } = b;
  stack("header", parent, "row", "fill", 100);
  stack("header-copy", "header", "column", "fill", 100);
  text("date", "header-copy", { maxWidth: 420 });
  gap("header-copy", 6);
  text("greeting", "header-copy", { maxWidth: 600 });
  gap("header-copy", 7);
  text("intro", "header-copy", { maxWidth: 570 });
  stack("header-action", "header", "column", 140, 100);
  gap("header-action", 19);
  stack("new-task-button", "header-action", "row", 140, 42, {
    fill: { styleId: "brand-fill" }, radius: 10, layout: { mode: "row", padding: 11, gap: 0 },
  });
  gap("new-task-button", 5, "row");
  text("new-task-label", "new-task-button", { maxWidth: 120 });
}

export function stats(b, parent) {
  const { stack, text, gap, inset } = b;
  stack("stats", parent, "row", "fill", 112, { layout: { mode: "row", padding: 0, gap: 20 } });
  for (const id of ["completed", "active", "focus"]) {
    const inner = inset(id, "stats", "fill", 112, 20, 16, {
      minWidth: 120, maxWidth: 248, fill: { styleId: "surface-fill" }, radius: 16,
    });
    text(`${id}-label`, inner, { maxWidth: 216 });
    gap(inner, 5);
    text(`${id}-value`, inner, { maxWidth: 216 });
    gap(inner, 5);
    text(`${id}-note`, inner, { maxWidth: 216 });
  }
}

export function tasks(b, parent) {
  const { stack, text, gap, old, inset } = b;
  const inner = inset("today-tasks", parent, "fill", 150, 22, 17, {
    fill: { styleId: "surface-fill" }, radius: 16,
  });
  stack("tasks-heading", inner, "row", "fill", 36);
  text("tasks-title", "tasks-heading");
  stack("tasks-count-column", "tasks-heading", "column", 180, 36);
  gap("tasks-count-column", 5);
  text("tasks-count", "tasks-count-column", { maxWidth: 170 });
  old("task-divider", inner, { width: "fill", maxWidth: 740 });
  gap(inner, 11);
  for (const id of ["review", "feedback"]) {
    stack(`${id}-row`, inner, "row", "fill", 41);
    stack(`${id}-check-column`, `${id}-row`, "column", 30, 41);
    gap(`${id}-check-column`, 4);
    old(`${id}-checkbox`, `${id}-check-column`);
    stack(`${id}-copy-column`, `${id}-row`, "column", "fill", 41);
    gap(`${id}-copy-column`, 2);
    text(`${id}-text`, `${id}-copy-column`, { maxWidth: 510 });
    stack(`${id}-status`, `${id}-row`, "row", 100, 26, {
      fill: id === "review" ? "#EEEDFF" : "#F2F4F7", radius: 13,
      layout: { mode: "row", padding: 5, gap: 0 },
    });
    gap(`${id}-status`, 9, "row");
    text(`${id}-status-label`, `${id}-status`, { maxWidth: 80 });
    gap(`${id}-row`, 22, "row");
  }
}
