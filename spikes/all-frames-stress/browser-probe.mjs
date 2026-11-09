import { observeBrowser } from "/__stress/browser-observe.mjs";
const config = window.__stressConfig,
  started = performance.now();
const { longTasks, errors, workerJobs } = observeBrowser();
const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
const stage = () =>
  window.Konva?.stages.find((entry) => entry.container()?.isConnected);
const frames = () => stage()?.find(".canvas-frame") ?? [];
function computedCounts(after = 0) {
  const nodes = new Map(
    workerJobs
      .slice(after)
      .filter((row) => row.kind === "compute" && row.nodes)
      .map((row) => [row.frameId, row.nodes]),
  );
  const total = [...nodes.values()].reduce((sum, count) => sum + count, 0);
  if (nodes.size !== 9 || total !== 10000)
    throw new Error(
      `Computed count mismatch: ${nodes.size} frames / ${total} nodes`,
    );
  return {
    frames: nodes.size,
    nodes: total,
    byFrame: Object.fromEntries(nodes),
  };
}
async function until(check, label) {
  const deadline = performance.now() + 180000;
  while (performance.now() < deadline) {
    if (check()) return;
    await pause(40);
  }
  throw new Error(`Timeout: ${label}`);
}
async function choose(label, value) {
  const button = document.querySelector(
    `[role="combobox"][aria-label="${label}"]`,
  );
  if (!button) throw new Error(`Missing ${label}`);
  button.click();
  await until(
    () => document.querySelector(`[role="option"][data-value="${value}"]`),
    `${label} option`,
  );
  document.querySelector(`[role="option"][data-value="${value}"]`).click();
}
async function ready(count, after = 0) {
  await until(() => {
    const computed = workerJobs
      .slice(after)
      .filter((row) => row.kind === "compute" && row.nodes);
    return (
      computed.length >= count &&
      frames().length === count &&
      !document.querySelector(".canvas-refresh,.canvas-message")
    );
  }, `${count} computed frames and canvas readiness`);
  await new Promise((resolve) =>
    requestAnimationFrame(() => requestAnimationFrame(resolve)),
  );
}
function board() {
  const host = document.querySelector(".canvas-host").getBoundingClientRect(),
    camera = stage();
  const rows = frames().map((frame) => {
    const rect = frame.findOne("Rect"),
      label = frame.findOne("Text");
    return {
      id: frame.id(),
      x: frame.x(),
      y: frame.y(),
      width: rect.width(),
      height: rect.height(),
      label: label?.text(),
      labelCssPx: label?.fontSize() * camera.scaleX(),
    };
  });
  const scale = camera.scaleX();
  return {
    frames: rows,
    scale,
    x: camera.x(),
    y: camera.y(),
    pane: { width: host.width, height: host.height },
    fits: rows.every(
      (row) =>
        camera.x() + row.x * scale >= -1 &&
        camera.y() + row.y * scale >= -1 &&
        camera.x() + (row.x + row.width) * scale <= host.width + 1 &&
        camera.y() + (row.y + row.height) * scale <= host.height + 1,
    ),
    overlap: rows.some((a, i) =>
      rows
        .slice(i + 1)
        .some(
          (b) =>
            a.x < b.x + b.width &&
            b.x < a.x + a.width &&
            a.y < b.y + b.height &&
            b.y < a.y + a.height,
        ),
    ),
  };
}
async function interact() {
  const host = document.querySelector(".canvas-host"),
    gaps = [];
  const canvas = host.querySelector("canvas"),
    initialScale = stage().scaleX();
  let maximumScale = initialScale;
  let previous = performance.now(),
    active = true;
  const sample = (now) => {
    gaps.push(now - previous);
    previous = now;
    if (active) requestAnimationFrame(sample);
  };
  requestAnimationFrame(sample);
  const begun = performance.now();
  for (let i = 0; i < 30; i++) {
    canvas.dispatchEvent(
      new WheelEvent("wheel", {
        bubbles: true,
        cancelable: true,
        deltaY: i < 15 ? -10 : 10,
        clientX: host.getBoundingClientRect().x + 100,
        clientY: 200,
      }),
    );
    host.dispatchEvent(
      new KeyboardEvent("keydown", {
        key: i % 2 ? "ArrowLeft" : "ArrowRight",
        bubbles: true,
      }),
    );
    await pause(16);
    maximumScale = Math.max(maximumScale, stage().scaleX());
  }
  active = false;
  return {
    elapsedMs: performance.now() - begun,
    rafCount: gaps.length,
    initialScale,
    maximumScale,
    zoomObserved: maximumScale > initialScale,
    maxRafGapMs: Math.max(...gaps),
    meanRafGapMs: gaps.reduce((a, b) => a + b, 0) / gaps.length,
  };
}
async function warm(layers) {
  if (!layers)
    document.querySelector('[aria-label="Hide layers panel"]').click();
  await choose("Frame", "frame-0");
  await pause(100);
  const offset = workerJobs.length,
    begun = performance.now();
  await choose("Frame", ":all:");
  await ready(9, offset);
  return {
    layersOpen: layers,
    computed: computedCounts(offset),
    readyMs: performance.now() - begun,
    layerRows: document.querySelectorAll(".layer-row").length,
    domElements: document.querySelectorAll("*").length,
    board: board(),
    interaction: await interact(),
    jobs: workerJobs.slice(offset),
  };
}
async function run() {
  await ready(9);
  const cold = {
    computed: computedCounts(),
    readyMs: performance.now() - started,
    layerRows: document.querySelectorAll(".layer-row").length,
    domElements: document.querySelectorAll("*").length,
    board: board(),
    jobs: [...workerJobs],
  };
  const warmOpen = await warm(true),
    warmClosed = await warm(false);
  const result = {
    profile: config.fixture.profile,
    userAgent: navigator.userAgent,
    hardwareConcurrency: navigator.hardwareConcurrency,
    viewport: { width: innerWidth, height: innerHeight, dpr: devicePixelRatio },
    cold,
    warmOpen,
    warmClosed,
    errors,
    longTasks,
    longTaskTotalMs: longTasks.reduce((sum, row) => sum + row.duration, 0),
    heap: performance.memory
      ? {
          used: performance.memory.usedJSHeapSize,
          total: performance.memory.totalJSHeapSize,
          limit: performance.memory.jsHeapSizeLimit,
        }
      : null,
    resources: performance
      .getEntriesByType("resource")
      .map(
        ({ name, duration, transferSize, decodedBodySize, initiatorType }) => ({
          name,
          duration,
          transferSize,
          decodedBodySize,
          initiatorType,
        }),
      ),
  };
  await fetch("/__stress/result", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify(result),
  });
  if (config.next) location.href = config.next;
  else {
    document.title = "Stress complete — 4 pages";
    window.__stressResult = result;
  }
}
run().catch(async (error) => {
  await fetch("/__stress/result", {
    method: "POST",
    headers: { "content-type": "application/json" },
    body: JSON.stringify({
      profile: config.fixture.profile,
      error: error.stack,
      workerJobs,
      errors,
      longTasks,
      elapsedMs: performance.now() - started,
    }),
  });
  document.title = "Stress failed";
});
