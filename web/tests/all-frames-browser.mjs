import assert from "node:assert/strict";
import { PAGE_SIZES } from "./all-frames-fixture.mjs";

export const choose = async (page, label, option) => {
  await page.getByRole("combobox", { name: label, exact: true }).click();
  await page.getByRole("option", { name: option, exact: true }).click();
};
export const frameScope = (page) =>
  page.getByRole("combobox", { name: "Frame", exact: true }).innerText();
export const settled = (page) => page.evaluate(() => new Promise((resolve) =>
  requestAnimationFrame(() => requestAnimationFrame(resolve)),
));
export const board = (page) => page.evaluate(() => {
  const stage = window.Konva.stages.find((entry) => entry.container()?.isConnected);
  if (!stage) return [];
  return stage.find(".canvas-frame").map((frame) => ({
    id: frame.id(), x: frame.x(), y: frame.y(),
    width: frame.findOne("Rect").width(),
    height: frame.findOne("Rect").height(),
    label: frame.findOne("Text")?.text(),
  }));
});
export const camera = (page) => page.evaluate(() => {
  const stage = window.Konva.stages.find((entry) => entry.container()?.isConnected);
  return { x: stage.x(), y: stage.y(), scale: stage.scaleX() };
});
export async function ready(page, count, revision = 7) {
  await page.getByText(count ? `Scene r${revision}` : "No scene", { exact: true }).waitFor();
  await settled(page);
  assert.equal((await board(page)).length, count);
}
export async function checkBoard(page, pageId) {
  const entries = await board(page);
  assert.equal(await frameScope(page), "All");
  assert.deepEqual(entries.map((entry) => entry.id),
    PAGE_SIZES[pageId].map((_, i) => `${pageId}-${i}`));
  for (const [index, entry] of entries.entries()) {
    assert.equal(entry.label, `${pageId} / Frame ${index + 1}`);
    assert.deepEqual([entry.width, entry.height], PAGE_SIZES[pageId][index]);
    assert.ok(entry.y >= 32);
    for (const other of entries) {
      if (entry.id === other.id) continue;
      assert.ok(
        entry.x + entry.width + 63 <= other.x ||
        other.x + other.width + 63 <= entry.x ||
        entry.y + entry.height + 95 <= other.y ||
        other.y + other.height + 95 <= entry.y,
      );
    }
  }
}

export async function checkServerPaths(browser, base, fixture, artifacts) {
  const server = await browser.newContext({ viewport: { width: 1440, height: 1000 } });
  try {
    const page = await server.newPage();
    // An explicit link requests single-frame scope, independent of the default All.
    await page.goto(`${base}/?scene=server&project=all-fixture&page=nine&frame=nine-0&node=nine-0-shape`);
    await ready(page, 1, 8);
    assert.equal(await frameScope(page), "nine / Frame 1");
    fixture.fail("nine-4");
    await choose(page, "Frame", "All");
    await page.getByText("Render failed", { exact: true }).first().waitFor();
    assert.equal((await board(page)).length, 9);
    await page.screenshot({ path: `${artifacts}/partial-failure.png` });
    fixture.fail("");
    await page.getByRole("button", { name: "Retry render" }).click();
    await ready(page, 9, 8);
    assert.deepEqual(fixture.metrics(), { maxActive: 4, overlap: false });
  } finally {
    fixture.fail("");
    await server.close();
  }
  const fallback = await browser.newContext();
  try {
    await fallback.route("**/scene-wasm/opencanvas_scene_wasm.js", (route) =>
      route.fulfill({ status: 404, body: "unavailable" }));
    const page = await fallback.newPage();
    await page.goto(base);
    await ready(page, 3, 8);
    await checkBoard(page, "three");
    await page.getByText(/Browser scene unavailable; using server scene/).waitFor();
    assert.deepEqual(fixture.metrics(), { maxActive: 4, overlap: false });
  } finally {
    await fallback.close();
  }
}
