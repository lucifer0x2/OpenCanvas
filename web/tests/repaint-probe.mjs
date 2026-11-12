// In-page counters for all-frames-repaint.mjs, installed with page.addInitScript(repaintProbe).
// Self-contained: it runs in the browser. Paint/index counts come from the build alias
// (`globalThis.__repaintCounts`, see repaint-viewer.mjs); this adds React commits, Konva layer
// draws, long tasks, retained offscreen canvas pixels and the pan/zoom drivers.
export function repaintProbe() {
  const frame = () => new Promise(requestAnimationFrame);
  const counts = () =>
    (globalThis.__repaintCounts ??= { paint: {}, paintTotal: 0, paintMs: 0, index: 0, inPaint: 0, publish: 0 });
  const state = { commits: {}, renderers: 0, draws: [], longTasks: [], canvases: [] };
  // React reports every commit to a devtools hook that exists before it loads.
  window.__REACT_DEVTOOLS_GLOBAL_HOOK__ = {
    supportsFiber: true,
    isDisabled: false,
    renderers: new Map(),
    inject: () => ++state.renderers,
    checkDCE() {},
    onScheduleFiberRoot() {},
    onCommitFiberUnmount() {},
    onPostCommitFiberRoot() {},
    // Renderer 1 is react-dom; react-konva's Stage reconciler injects second.
    onCommitFiberRoot: (id) => void (state.commits[id] = (state.commits[id] ?? 0) + 1),
  };
  new PerformanceObserver((list) =>
    state.longTasks.push(...list.getEntries().map((e) => e.duration)),
  ).observe({ type: "longtask" });
  // Canvases created outside paintScene (opacity layers there are transient per draw).
  const create = Document.prototype.createElement;
  Document.prototype.createElement = function (name, ...rest) {
    const element = create.call(this, name, ...rest);
    if (String(name).toLowerCase() === "canvas" && !counts().inPaint)
      state.canvases.push(element);
    return element;
  };
  let wrapped = false;
  const install = () => {
    if (wrapped) return;
    wrapped = true;
    const Layer = window.Konva.Layer.prototype,
      draw = Layer.drawScene;
    Layer.drawScene = function (...args) {
      const t = performance.now();
      try {
        return draw.apply(this, args);
      } finally {
        state.draws.push(performance.now() - t);
      }
    };
  };
  const waitKonva = setInterval(() => {
    if (!window.Konva) return;
    clearInterval(waitKonva);
    install();
  }, 5);
  const stage = () => window.Konva.stages.find((s) => s.container()?.isConnected);
  /** Retained pixels of offscreen canvases that are not Konva's own stage/layer canvases. */
  const offscreen = () => {
    const konva = new Set();
    for (const s of window.Konva.stages) {
      for (const c of [s.bufferCanvas, s.bufferHitCanvas]) if (c) konva.add(c._canvas);
      for (const l of s.getLayers())
        for (const c of [l.getCanvas(), l.getHitCanvas()]) konva.add(c._canvas);
    }
    const live = state.canvases.filter(
      (c) => !konva.has(c) && !c.isConnected && c.width * c.height > 1,
    );
    return { canvases: live.length, pixels: live.reduce((n, c) => n + c.width * c.height, 0) };
  };
  const stats = (values) => {
    const sorted = [...values].sort((a, b) => a - b);
    const sum = sorted.reduce((a, b) => a + b, 0);
    return {
      n: sorted.length,
      total: sum,
      mean: sorted.length ? sum / sorted.length : 0,
      p95: sorted[Math.min(sorted.length - 1, Math.floor(0.95 * sorted.length))] ?? 0,
      max: sorted.at(-1) ?? 0,
    };
  };
  function reset() {
    install();
    const c = counts();
    Object.assign(c, { paint: {}, paintTotal: 0, paintMs: 0, index: 0, publish: 0 });
    state.commits = {};
    state.draws.length = 0;
    state.longTasks.length = 0;
  }
  function read() {
    const c = counts();
    return {
      paintTotal: c.paintTotal,
      paintMs: c.paintMs,
      paintPerFrame: { ...c.paint },
      index: c.index,
      publishes: c.publish,
      reactCommits: state.commits[1] ?? 0,
      konvaCommits: state.commits[2] ?? 0,
      layerDraws: stats(state.draws),
      longTasks: stats(state.longTasks),
      offscreen: offscreen(),
    };
  }
  /** Resolves with Date.now() when the status first names revision `r` as the current scene. */
  function whenCurrent(r) {
    const done = () => [...document.querySelectorAll(".statusbar span")].some(
      (e) => e.textContent === `Scene r${r}`,
    );
    return new Promise((resolve) => {
      if (done()) return resolve(Date.now());
      const observer = new MutationObserver(() => {
        if (!done()) return;
        observer.disconnect();
        resolve(Date.now());
      });
      observer.observe(document.querySelector(".statusbar") ?? document.body, { subtree: true, childList: true, characterData: true });
    });
  }
  /** One arrow-key pan step per animation frame; frame intervals and layer draws. */
  async function pan(steps) {
    const host = document.querySelector(".canvas-host");
    await frame();
    reset();
    const intervals = [];
    let last = await frame();
    for (let i = 0; i < steps; i++) {
      const key = i < steps / 2 ? "ArrowLeft" : "ArrowRight";
      host.dispatchEvent(new KeyboardEvent("keydown", { key, bubbles: true }));
      const now = await frame();
      intervals.push(now - last);
      last = now;
    }
    await frame();
    return { frameIntervals: stats(intervals), ...read() };
  }
  /** Wheel-zoom about the pane centre to `target` scale over a few frames. */
  async function zoomTo(target) {
    const el = document.querySelector(".konvajs-content"),
      rect = el.getBoundingClientRect();
    for (let i = 0; i < 80 && Math.abs(stage().scaleX() / target - 1) > 0.001; i++) {
      const delta = Math.max(-80, Math.min(80, -Math.log(target / stage().scaleX()) / 0.0012));
      el.dispatchEvent(new WheelEvent("wheel", {
        deltaY: delta, clientX: rect.left + rect.width / 2, clientY: rect.top + rect.height / 2,
        bubbles: true, cancelable: true,
      }));
      await frame();
    }
    return stage().scaleX();
  }
  /** Arrow-key pan (40 px steps) until the frame's centre is near the pane centre. */
  async function centerOn(id) {
    const s = stage(),
      group = s.findOne(`#${id}`),
      rect = group.findOne("Rect");
    const at = (axis, size) =>
      s[axis]() + (group[axis]() + rect[size]() / 2) * s.scaleX();
    for (const [axis, size, pane, keys] of [
      ["x", "width", s.width(), ["ArrowLeft", "ArrowRight"]],
      ["y", "height", s.height(), ["ArrowUp", "ArrowDown"]],
    ]) {
      const steps = Math.round((pane / 2 - at(axis, size)) / 40);
      for (let i = 0; i < Math.abs(steps); i++)
        document.querySelector(".canvas-host").dispatchEvent(
          new KeyboardEvent("keydown", { key: keys[steps > 0 ? 0 : 1], bubbles: true }),
        );
      await frame();
    }
  }
  /** The layer's device pixels, base64 RGBA, for a cross-build comparison at rest. */
  function pixels() {
    const canvas = stage().getLayers()[0].getCanvas()._canvas;
    const data = canvas.getContext("2d").getImageData(0, 0, canvas.width, canvas.height).data;
    let binary = "";
    for (let i = 0; i < data.length; i += 0x8000)
      binary += String.fromCharCode(...data.subarray(i, i + 0x8000));
    return { width: canvas.width, height: canvas.height, rgba: btoa(binary) };
  }
  window.__repaint = {
    reset, read, whenCurrent, pan, zoomTo, centerOn, pixels, frame,
    scale: () => stage().scaleX(),
  };
}
