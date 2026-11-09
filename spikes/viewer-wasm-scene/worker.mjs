// This module runs only in a real Chromium dedicated module Worker.
self.onmessage = async ({ data: fixture }) => {
  try {
    let start = performance.now();
    const module = await import("/scene-wasm/opencanvas_scene_wasm.js");
    const importMs = performance.now() - start;
    start = performance.now();
    await module.default({
      module_or_path: "/scene-wasm/opencanvas_scene_wasm_bg.wasm",
    });
    const initMs = performance.now() - start;
    const computer = new module.SceneComputer();
    start = performance.now();
    const required = JSON.parse(
      computer.required_fonts(
        JSON.stringify(fixture.snapshot),
        fixture.frameId,
        fixture.width,
        fixture.height,
        "default",
      ),
    );
    const discoveryMs = performance.now() - start;
    const fontTimings = [];
    for (const font of required) {
      const captured = fixture.fonts.find(
        (candidate) => candidate.id === font.id,
      );
      if (!captured || captured.hash !== font.hash)
        throw new Error(`Uncaptured required font ${font.id}`);
      start = performance.now();
      const response = await fetch(`/artifacts/${captured.file}`);
      if (!response.ok) throw new Error(`Font fetch ${response.status}`);
      const bytes = new Uint8Array(await response.arrayBuffer());
      const fetchMs = performance.now() - start;
      start = performance.now();
      computer.load_font(font.id, font.hash, bytes);
      fontTimings.push({
        id: font.id,
        hash: font.hash,
        bytes: bytes.length,
        fetchMs,
        decodeMs: performance.now() - start,
      });
    }
    computer.set_resource_urls(
      JSON.stringify({
        fonts: Object.fromEntries(
          fixture.native.scene.fonts.map((f) => [f.id, f.url]),
        ),
        assets: Object.fromEntries(
          fixture.native.scene.assets.map((a) => [a.id, a.url]),
        ),
      }),
    );
    const compute = () => {
      const started = performance.now();
      // Includes JS serialization, Rust snapshot parse/resolution/layout/text/scene JSON,
      // and JS scene parse. Worker message transport and resources are outside this interval.
      const scene = JSON.parse(
        computer.compute_scene(
          JSON.stringify(fixture.snapshot),
          fixture.frameId,
          fixture.width,
          fixture.height,
          "default",
        ),
      );
      return { scene, ms: performance.now() - started };
    };
    const cold = compute();
    const warm = [];
    for (let i = 0; i < 20; i++) {
      const next = compute();
      if (JSON.stringify(next.scene) !== JSON.stringify(cold.scene))
        throw new Error("Warm nondeterminism");
      warm.push(next.ms);
    }
    computer.free();
    self.postMessage({
      scene: cold.scene,
      timings: {
        importMs,
        initMs,
        discoveryMs,
        fonts: fontTimings,
        coldMs: cold.ms,
        warmMs: warm,
      },
      worker: {
        userAgent: navigator.userAgent,
        hardwareConcurrency: navigator.hardwareConcurrency,
        crossOriginIsolated: self.crossOriginIsolated,
        timerResolutionNote: "performance.now; local HTTP without COOP/COEP",
      },
    });
  } catch (error) {
    self.postMessage({
      error: { message: error.message, code: error.code, stack: error.stack },
    });
  }
};
