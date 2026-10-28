import { BUNDLED_FONT } from "../../worker/pipeline.mjs";

const node = (id, parentId, kind, props) => ({ id, parentId, kind, props });

export function responsiveFixture(
  width,
  { text = false, wrap = true, min = 280 } = {},
) {
  const nodes = [
    node("frame", null, "frame", {
      width: 1040,
      height: 844,
      fill: "#ffffff",
      layout: { mode: "column", padding: 24, gap: 16 },
    }),
    node("cards", "frame", "group", {
      width: "fill",
      height: "hug",
      layout: { mode: "row", gap: 16, wrap },
    }),
  ];
  for (let index = 0; index < 3; index++) {
    const id = `card-${index + 1}`;
    nodes.push(
      node(id, "cards", "group", {
        width: "fill",
        height: text ? "hug" : 40 + index * 20,
        ...(min === null ? {} : { minWidth: min }),
        fill: ["#dbeafe", "#dcfce7", "#fef3c7"][index],
        layout: { mode: "column", padding: text ? 16 : 0, gap: 8 },
      }),
    );
    if (text)
      nodes.push(
        node(`${id}-text`, id, "text", {
          text: [
            "Một thiết kế có thể xuống dòng tự nhiên khi chiều rộng thay đổi.",
            "Nội dung dài hơn để kiểm tra chiều cao ô và hàng, tiếng Việt có dấu, không cắt chữ hoặc vẽ chồng lên hàng kế tiếp.",
            "OpenCanvasTextBoundaryGraphemeTiếngViệtKhôngKhoảngTrắng0123456789 và nội dung cuối.",
          ][index],
          width: "fill",
          height: "hug",
          fontId: "font_bundled",
          fontSize: 16,
          lineHeight: 24,
          fill: "#182230",
        }),
      );
  }
  return {
    schemaVersion: 1,
    projectId: "wrap-fixture",
    revision: 1,
    frameId: "frame",
    viewport: { width, height: 844 },
    theme: "default",
    state: "default",
    fonts: [{ ...BUNDLED_FONT }],
    assets: [],
    nodes,
  };
}
