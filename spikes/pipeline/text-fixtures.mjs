import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
export async function textFonts() {
  return Promise.all([["font_bundled", "renderer/fonts/NotoSans.ttf"], ["lato", "fixtures/fonts/Lato-Regular.ttf"]].map(async ([id, file]) => {
    const bytes = await readFile(file);
    return { id, family: id, weight: 400, style: "normal", hash: createHash("sha256").update(bytes).digest("hex"), url: `data:font/ttf;base64,${bytes.toString("base64")}` };
  }));
}
export function textInput(fonts, props = {}) {
  return { schemaVersion: 1, projectId: "text-test", revision: 1, frameId: "frame", viewport: { width: 400, height: 160 }, fonts, assets: [], nodes: [
    { id: "frame", parentId: null, kind: "frame", props: { width: 400, height: 160, fill: "#fff", clip: false } },
    { id: "text", parentId: "frame", kind: "text", props: { x: 10, y: 10, width: 300, fontId: fonts[0].id, fontSize: 14, lineHeight: 20, text: "Sao chép Ấp\nsecond line ỹ", fill: "#182230", ...props } },
  ] };
}
export const copy = ["⌘C", "⇧⌘V", "Prefer", "customers", "Sao chép", "Filter objects", "Hello world", "Tiếng Việt ă â đ ê ô ơ ư", "Ấp Sao chép", "second line ỹ", "á", "Ắ", "Đặng", "OpenCanvas", "Settings", "Start", "123.45", "-0.05", "Inbox (12)", "File > Open", "A&B", "Save…", "100%", "UI / UX", "Name", "Small caps", "ﬁ ﬂ", "straße", "Delete", "Preferences", "Search", "View details", "Restore", "Next", "Cancel", "Submit", "Read only", "Project", "Diagnostics", "Font family"];
