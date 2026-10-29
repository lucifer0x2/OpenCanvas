// Motion preview fixture, created through the real MCP API of a temporary server.
const node = (id, kind, parentId, props) => ({
  op: "create_node",
  id,
  kind,
  parentId,
  props,
});
const column = { mode: "column", padding: 24, gap: 16 };

export const FIXTURE = {
  load: { id: "hero", name: "enter", delay: 300, duration: 400 },
  hover: {
    id: "card",
    name: "lift",
    duration: 200,
    scale: 1.1,
    fill: "rgb(255, 0, 0)",
    rest: "rgb(238, 238, 238)",
  },
  click: { id: "pulse", name: "pulse", duration: 300 },
  link: { id: "go", to: "second" },
  back: { id: "back" },
};

export async function createFixture(mcp) {
  const { projectId } = await mcp.call("create_project", {
    slug: "motion-preview-smoke",
    name: "Motion preview smoke fixture",
  });
  const operations = [
    {
      op: "upsert_token",
      token: { id: "brand", name: "Brand", type: "color", value: "#2266cc" },
    },
    {
      op: "upsert_token",
      token: { id: "lift", name: "Lift", type: "number", value: 24 },
    },
    node("page", "page", null, { name: "Flow" }),
    node("home", "frame", "page", {
      name: "Home",
      width: 480,
      height: 420,
      fill: "#ffffff",
      layout: column,
    }),
    node("hero", "rectangle", "home", {
      name: "Hero",
      width: "fill",
      height: 56,
      fill: { tokenId: "brand" },
      animations: [
        {
          name: "enter",
          duration: FIXTURE.load.duration,
          delay: FIXTURE.load.delay,
          keyframes: [
            { offset: 0, opacity: 0, translateY: { tokenId: "lift" } },
          ],
        },
      ],
    }),
    node("card", "rectangle", "home", {
      name: "Card",
      width: 200,
      height: 60,
      radius: 8,
      fill: "#eeeeee",
      states: { hover: { fill: "#ff0000" } },
      animations: [
        {
          name: "lift",
          on: "hover",
          duration: FIXTURE.hover.duration,
          keyframes: [{ scale: FIXTURE.hover.scale }],
        },
      ],
    }),
    node("pulse", "rectangle", "home", {
      name: "Pulse",
      width: 120,
      height: 40,
      radius: 6,
      fill: "#10b981",
      stroke: { color: "#334155", width: 3, align: "outside" },
      states: { pressed: { opacity: 0.6 } },
      animations: [
        {
          name: "pulse",
          on: "click",
          duration: FIXTURE.click.duration,
          keyframes: [{ offset: 0.5, scale: 0.9 }],
        },
      ],
    }),
    node("go", "rectangle", "home", {
      name: "Go",
      width: 120,
      height: 40,
      fill: "#111827",
      links: [{ to: "second" }],
    }),
    node("title", "text", "home", {
      name: "Title",
      text: "Motion preview",
      width: "fill",
      height: "hug",
      fontId: "font_bundled",
      fontSize: 24,
      lineHeight: 32,
      fill: "#15263b",
      animations: [
        {
          name: "tint",
          duration: 300,
          keyframes: [{ offset: 0, fill: { tokenId: "brand" } }],
        },
      ],
    }),
    node("second", "frame", "page", {
      name: "Second",
      width: 480,
      height: 420,
      fill: "#f8fafc",
      layout: column,
    }),
    node("back", "rectangle", "second", {
      name: "Back",
      width: 120,
      height: 40,
      fill: "#f59e0b",
      links: [{ action: "back" }],
    }),
  ];
  await mcp.call("apply_batch", {
    projectId,
    expectedRevision: 0,
    idempotencyKey: "motion-fixture",
    operations,
  });
  return projectId;
}
