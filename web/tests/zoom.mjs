// Zoom sharpness, memory and frame cost of the viewer canvas (proposal P01, AC1-AC5 and AC7).
// Measurements run in the page (zoom-probe.mjs) against fixture scenes (zoom-harness.mjs).
import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";
import { open, withViewer } from "./zoom-harness.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);
const { chromium } = await import("playwright");
const rendererUrl = `/@fs${fileURLToPath(new URL("../../renderer/index.ts", import.meta.url))}`;
// Wheel steps from fit: x3.6 overall, so the card reaches 360% and the dashboard ~211%.
const STEPS = [
  [1.2, 200],
  [2, 600],
  [1.5, 300],
];
const rest = (page) => page.waitForTimeout(1500);

/** AC1-AC3 and the wheel anchor half of AC7: compare at mid-gesture and at rest after each step. */
async function sharpness(browser, scene, dpr, extraSteps = []) {
  const { page, errors, sceneUrl } = await open(browser, dpr);
  await page.evaluate(
    ([r, s]) => window.__zoom.setup(r, s),
    [rendererUrl, sceneUrl()],
  );
  const samples = [
    {
      step: "fit",
      rest: await page.evaluate(() => window.__zoom.compare("rest")),
    },
  ];
  for (const [factor, ms] of [...STEPS, ...extraSteps]) {
    const moved = await page.evaluate(
      ([f, t]) => window.__zoom.gesture(f, t),
      [factor, ms],
    );
    const mid = await page.evaluate(() => window.__zoom.compare("mid"));
    await rest(page);
    const after = await page.evaluate(() => window.__zoom.compare("rest"));
    samples.push({
      step: `x${factor}/${ms}ms`,
      drift: moved.drift,
      mid,
      rest: after,
    });
  }
  assert.deepEqual(errors, []);
  await page.close();
  return samples.map((s) => ({ scene, dpr, ...s }));
}

/** AC4: no new canvas and no backing store larger than the stage during and after a gesture. */
async function memory(browser, scene) {
  const { page, errors } = await open(browser, 2);
  const result = await page.evaluate(async () => {
    const watch = window.__zoom.watchCanvases();
    await window.__zoom.gesture(4, 1000);
    await new Promise((resolve) => setTimeout(resolve, 1500));
    const st = window.Konva.stages[0];
    return {
      ...watch,
      stagePixels:
        st.width() * devicePixelRatio * st.height() * devicePixelRatio,
    };
  });
  assert.deepEqual(errors, []);
  await page.close();
  return { scene, ...result };
}

/** AC5: Konva layer draw time and long tasks across a zoom in and back out. */
async function cost(browser, scene) {
  const { page, errors } = await open(browser, 2);
  const result = await page.evaluate(async () => {
    const watch = window.__zoom.watchFrames();
    await window.__zoom.gesture(4, 1000);
    await window.__zoom.gesture(0.25, 600);
    await new Promise((resolve) => setTimeout(resolve, 100));
    const sorted = [...watch.draws].sort((a, b) => a - b);
    const at = (q) =>
      sorted[Math.min(sorted.length - 1, Math.floor(q * sorted.length))];
    return {
      draws: sorted.length,
      p50: at(0.5),
      p95: at(0.95),
      max: sorted.at(-1),
      longTasks: watch.longTasks,
    };
  });
  assert.deepEqual(errors, []);
  await page.close();
  return { scene, ...result };
}

/** AC7: at ~200% a click on the title's centre still selects it. */
async function clickAtZoom(browser) {
  const { page, errors } = await open(browser, 2);
  await page.evaluate(() => window.__zoom.gesture(2, 600));
  const point = await page.evaluate(() => {
    const st = window.Konva.stages[0],
      rect = document.querySelector(".konvajs-content").getBoundingClientRect();
    return {
      zoom: st.scaleX(),
      x: rect.left + st.x() + (56 + 325) * st.scaleX(),
      y: rect.top + st.y() + (102 + 54) * st.scaleY(),
    };
  });
  await page.mouse.click(point.x, point.y);
  await page
    .getByRole("heading", { name: "Headline", exact: true })
    .waitFor({ timeout: 5000 });
  assert.deepEqual(errors, []);
  await page.close();
  return { zoom: point.zoom, selected: "Headline" };
}

const browser = await chromium.launch({ headless: true });
const report = {
  browser: browser.version(),
  sharpness: [],
  memory: [],
  cost: [],
};
try {
  await withViewer("light", async () => {
    report.sharpness.push(
      ...(await sharpness(browser, "light", 2, [[1.2, 200]])),
    );
    report.sharpness.push(...(await sharpness(browser, "light", 1)));
    report.memory.push(await memory(browser, "light"));
    report.click = await clickAtZoom(browser);
  });
  await withViewer("dense", async () => {
    report.sharpness.push(...(await sharpness(browser, "dense", 2)));
    report.sharpness.push(...(await sharpness(browser, "dense", 1)));
    report.memory.push(await memory(browser, "dense"));
  });
  await withViewer("stress", async () => {
    report.cost.push(await cost(browser, "stress"));
  });
} finally {
  await browser.close();
}
console.log(JSON.stringify(report, null, 2));

const label = (s) =>
  `${s.scene} dpr${s.dpr} ${s.step} @${(s.rest.zoom * 100).toFixed(0)}%`;
for (const s of report.sharpness) {
  assert.ok(
    s.rest.sharp >= 0.95,
    `AC3 rest sharp ${label(s)}: ${s.rest.sharp}`,
  );
  if (!s.mid) continue;
  assert.ok(s.mid.sharp >= 0.95, `AC1 mid sharp ${label(s)}: ${s.mid.sharp}`);
  assert.ok(s.mid.off <= 0.005, `AC1 mid off ${label(s)}: ${s.mid.off}`);
  assert.ok(
    s.rest.changedAfterRest <= 0.001,
    `AC2 changed after rest ${label(s)}: ${s.rest.changedAfterRest}`,
  );
  assert.ok(
    s.drift <= 1 + 1e-6,
    `AC7 wheel anchor drift ${label(s)}: ${s.drift}px`,
  );
}
assert.ok(
  report.sharpness.some(
    (s) => s.scene === "light" && s.dpr === 2 && s.rest.zoom === 4,
  ),
  "AC3 reaches MAX_SCALE 4",
);
for (const m of report.memory) {
  assert.equal(m.created, 0, `AC4 canvases created (${m.scene})`);
  assert.ok(
    m.maxPixels <= m.stagePixels,
    `AC4 backing store ${m.maxPixels} > ${m.stagePixels} (${m.scene})`,
  );
}
for (const c of report.cost) {
  assert.ok(c.draws > 30, `AC5 layer draws observed: ${c.draws}`);
  assert.ok(c.p95 <= 8 && c.max <= 16, `AC5 draw p95 ${c.p95} max ${c.max} ms`);
  assert.equal(c.longTasks, 0, "AC5 long tasks");
}
console.log("zoom: AC1-AC5, AC7 pass");
