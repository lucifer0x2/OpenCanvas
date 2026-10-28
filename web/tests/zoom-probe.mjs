// In-page measurement helpers for tests/zoom.mjs. Installed with page.addInitScript(zoomProbe), so
// the function must stay self-contained: it runs in the browser, not in Node.
export function zoomProbe() {
  const frame = () => new Promise(requestAnimationFrame);
  const luma = (d, i) => 0.299 * d[i] + 0.587 * d[i + 1] + 0.114 * d[i + 2];
  /** Mean squared luma gradient: drops as an image is blurred by resampling. */
  const sharpness = (w, h, d) => {
    let s = 0;
    for (let y = 0; y < h - 1; y++)
      for (let x = 0; x < w - 1; x++) {
        const i = (y * w + x) * 4;
        const gx = luma(d, i + 4) - luma(d, i);
        const gy = luma(d, i + w * 4) - luma(d, i);
        s += gx * gx + gy * gy;
      }
    return s / (w * h);
  };
  const differing = (a, b, threshold) => {
    let off = 0,
      n = 0;
    for (let i = 0; i < a.length; i += 4)
      for (let j = 0; j < 3; j++, n++)
        if (Math.abs(a[i + j] - b[i + j]) > threshold) off++;
    return off / n;
  };
  const content = () => document.querySelector(".konvajs-content");
  const state = { R: null, scene: null, resources: null, mid: null };

  /** Draws `ms` of wheel zoom by `factor`, one event per frame, like a trackpad; ends at "mid". */
  async function gesture(factor, ms) {
    const el = content(),
      rect = el.getBoundingClientRect();
    const frames = Math.max(1, Math.round(ms / (1000 / 60)));
    const deltaY = -Math.log(factor) / frames / 0.0012;
    const clientX = rect.left + rect.width / 2,
      clientY = rect.top + rect.height / 2;
    const st = window.Konva.stages[0];
    const before = { x: st.x(), y: st.y(), scale: st.scaleX() };
    for (let i = 0; i < frames; i++) {
      await frame();
      el.dispatchEvent(
        new WheelEvent("wheel", {
          deltaY,
          clientX,
          clientY,
          bubbles: true,
          cancelable: true,
        }),
      );
    }
    await frame();
    await frame();
    // Scene point that was under the pointer, mapped through the new transform (CSS px).
    const px = rect.width / 2,
      py = rect.height / 2;
    const drift = Math.max(
      Math.abs(st.x() + ((px - before.x) / before.scale) * st.scaleX() - px),
      Math.abs(st.y() + ((py - before.y) / before.scale) * st.scaleY() - py),
    );
    return { zoom: st.scaleX(), drift };
  }

  async function setup(rendererUrl, sceneUrl) {
    state.R = await import(rendererUrl);
    state.scene = (await (await fetch(sceneUrl)).json()).scene;
    state.resources = await state.R.loadResources(
      state.scene.fonts,
      state.scene.assets,
      { strict: false },
    );
  }

  /** On-screen crop of the frame vs drawScene of the same scene window at zoom * DPR. */
  function compare(phase) {
    const { R, scene, resources } = state;
    const st = window.Konva.stages[0],
      zoom = st.scaleX(),
      dpr = devicePixelRatio;
    const vp = scene.provenance.viewport;
    const c = content().querySelector("canvas");
    const fx0 = Math.max(0, Math.ceil(st.x() * dpr) + 2),
      fy0 = Math.max(0, Math.ceil(st.y() * dpr) + 2);
    const fx1 = Math.min(
      c.width,
      Math.floor((st.x() + vp.width * zoom) * dpr) - 2,
    );
    const fy1 = Math.min(
      c.height,
      Math.floor((st.y() + vp.height * zoom) * dpr) - 2,
    );
    const w = Math.min(1200, fx1 - fx0),
      h = Math.min(800, fy1 - fy0);
    const x0 = Math.floor((fx0 + fx1 - w) / 2),
      y0 = Math.floor((fy0 + fy1 - h) / 2);
    const shown = c.getContext("2d").getImageData(x0, y0, w, h).data;
    // drawScene caps scale at 4: above it, scale scene coordinates by k and draw at s / k.
    const s = zoom * dpr,
      k = s > 4 ? Math.ceil(s / 4) : 1;
    const ox = (x0 / dpr - st.x()) / zoom,
      oy = (y0 / dpr - st.y()) / zoom;
    const win = {
      ...scene,
      provenance: {
        ...scene.provenance,
        viewport: { width: (w / s) * k, height: (h / s) * k },
      },
      nodes: scene.nodes.map((n) => ({
        ...n,
        fontSize: n.fontSize * k,
        lineHeight: n.lineHeight * k,
        radius: (n.radius ?? 0) * k,
        bounds: {
          x: (n.bounds.x - ox) * k,
          y: (n.bounds.y - oy) * k,
          width: n.bounds.width * k,
          height: n.bounds.height * k,
        },
        lines: n.lines.map((l) => ({
          ...l,
          x: l.x * k,
          y: l.y * k,
          width: l.width * k,
        })),
      })),
    };
    const ref = document.createElement("canvas");
    R.drawScene(ref, win, resources, { scale: s / k, strict: false });
    const rd = ref.getContext("2d").getImageData(0, 0, w, h).data;
    const result = {
      zoom,
      sharp: sharpness(w, h, shown) / sharpness(w, h, rd),
      off: differing(shown, rd, 16),
    };
    if (phase === "mid") state.mid = shown;
    else if (state.mid?.length === shown.length)
      result.changedAfterRest = differing(state.mid, shown, 8);
    return result;
  }

  /** Counts canvases created and the largest backing store set while `run` is pending. */
  function watchCanvases() {
    const watch = { created: 0, maxPixels: 0 };
    const create = Document.prototype.createElement;
    Document.prototype.createElement = function (name, ...rest) {
      if (String(name).toLowerCase() === "canvas") watch.created++;
      return create.call(this, name, ...rest);
    };
    for (const key of ["width", "height"]) {
      const desc = Object.getOwnPropertyDescriptor(
        HTMLCanvasElement.prototype,
        key,
      );
      Object.defineProperty(HTMLCanvasElement.prototype, key, {
        ...desc,
        set(value) {
          desc.set.call(this, value);
          watch.maxPixels = Math.max(watch.maxPixels, this.width * this.height);
        },
      });
    }
    return watch;
  }

  /** Durations of every Konva layer draw, and long tasks, while measuring. */
  function watchFrames() {
    const watch = { draws: [], longTasks: 0 };
    const Layer = window.Konva.Layer.prototype,
      draw = Layer.drawScene;
    Layer.drawScene = function (...args) {
      const t = performance.now();
      try {
        return draw.apply(this, args);
      } finally {
        watch.draws.push(performance.now() - t);
      }
    };
    new PerformanceObserver(
      (list) => (watch.longTasks += list.getEntries().length),
    ).observe({ type: "longtask" });
    return watch;
  }

  window.__zoom = {
    gesture,
    setup,
    compare,
    watchCanvases,
    watchFrames,
    frame,
  };
}
