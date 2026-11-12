// Count actual row/frame function executions through Vite's test-only transform.
// Production modules carry no counters; the browser still runs the real React/Konva tree.
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { createRequire } from "node:module";
import { startAllFramesFixture } from "./all-frames-fixture.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(new URL("../../.runtime/browsers", import.meta.url));
const root = fileURLToPath(new URL("../", import.meta.url));
const require = createRequire(new URL("../package.json", import.meta.url));
const { createServer } = await import(require.resolve("vite"));
const fixture = await startAllFramesFixture();
let vite, browser;
try {
  vite = await createServer({
    root,
    logLevel: "warn",
    server: { host: "127.0.0.1", port: 0, proxy: { "/api": { target: fixture.baseURL } } },
    plugins: [{
      name: "selection-render-counts",
      enforce: "pre",
      transform(source, id) {
        const kind = id.endsWith("/components/LayerRow.tsx") ? "rows"
          : id.endsWith("/components/CanvasFrame.tsx") ? "frames"
          : id.endsWith("/components/CanvasView.tsx") ? "views" : undefined;
        if (!kind) return;
        const marker = kind === "rows" ? "  const [expanded, setExpanded]"
          : kind === "frames" ? "  const selected =" : "  const stage =";
        assert.ok(source.includes(marker), `render probe marker in ${id}`);
        const key = kind === "rows" ? "node.id" : kind === "frames" ? "frame.id" : '"canvas"';
        const counter = `  const counts = (globalThis.__selectionRenders ??= { rows: {}, frames: {}, views: {} }).${kind};\n  counts[${key}] = (counts[${key}] ?? 0) + 1;\n`;
        return source.replace(marker, counter + marker);
      },
    }],
  });
  await vite.listen();
  const { chromium } = await import("playwright");
  browser = await chromium.launch({ headless: true });
  const page = await browser.newPage({ viewport: { width: 1440, height: 1000 } });
  const errors = [];
  page.on("pageerror", (error) => errors.push(error.message));
  const choose = async (label, name) => {
    await page.getByRole("combobox", { name: label, exact: true }).click();
    await page.getByRole("option", { name, exact: true }).click();
  };
  const settle = () => page.evaluate(() => new Promise((resolve) => requestAnimationFrame(() => requestAnimationFrame(resolve))));
  const reset = () => page.evaluate(() => { globalThis.__selectionRenders = { rows: {}, frames: {}, views: {} }; });
  const counts = () => page.evaluate(() => globalThis.__selectionRenders);
  await page.goto(`http://127.0.0.1:${vite.httpServer.address().port}/?scene=server`);
  await page.getByText("Scene r7", { exact: true }).waitFor();
  await choose("Page", "nine");
  await choose("Frame", "All");
  await page.getByText("Scene r7", { exact: true }).waitFor();
  await settle();
  assert.equal(await page.locator(".layer-select").count(), 27);
  await page.evaluate(() => { window.__selectionStage = window.Konva.stages[0]; });
  await reset();
  await page.getByRole("button", { name: "Show notice bar", exact: true }).click();
  await settle();
  assert.deepEqual(await counts(), { rows: {}, frames: {}, views: {} }, "unrelated chrome preference leaves canvas and layers idle");
  await reset();
  await page.getByRole("button", { name: "Accent 2", exact: true }).click();
  await settle();
  let rendered = await counts();
  assert.ok(Object.keys(rendered.rows).length <= 4, JSON.stringify(rendered));
  assert.ok(Object.keys(rendered.rows).every((id) => /^nine-[01](?:-|$)/.test(id)), JSON.stringify(rendered));
  assert.ok(Object.keys(rendered.frames).length <= 2, JSON.stringify(rendered));
  await reset();
  await page.getByRole("button", { name: "Title 2", exact: true }).click();
  await settle();
  rendered = await counts();
  assert.deepEqual(Object.keys(rendered.rows).sort(), ["nine-1", "nine-1-shape", "nine-1-title"]);
  assert.deepEqual(Object.keys(rendered.frames), ["nine-1"]);
  const title = page.getByRole("button", { name: "Title 2", exact: true });
  assert.equal(await title.getAttribute("aria-pressed"), "true");
  await page.getByRole("button", { name: "Collapse nine / Frame 9", exact: true }).click();
  await title.click();
  await title.press("Shift+F10");
  await page.getByRole("menu", { name: "Element actions" }).waitFor();
  assert.equal(await page.getByRole("menuitem", { name: /Copy link/ }).evaluate((el) => el === document.activeElement), true);
  await page.keyboard.press("Escape");
  assert.equal(await title.evaluate((el) => el === document.activeElement), true);
  await page.getByRole("button", { name: "Accent 3", exact: true }).click({ button: "right" });
  await page.getByRole("menu", { name: "Element actions" }).waitFor();
  assert.equal(await page.getByRole("button", { name: "Accent 3", exact: true }).getAttribute("aria-pressed"), "true");
  await page.keyboard.press("Escape");
  assert.equal(await page.getByRole("button", { name: "Expand nine / Frame 9", exact: true }).count(), 1);
  assert.equal(await page.evaluate(() => window.Konva.stages[0] === window.__selectionStage), true);
  assert.deepEqual(errors, []);
  console.log("Layers render smoke passed: 27 visible rows → 3 rows and 1 frame on sibling selection; notice toggle renders no canvas/layers; expansion, keyboard/context actions and Stage identity preserved.");
} finally {
  await browser?.close();
  await vite?.close();
  await fixture.close();
}
