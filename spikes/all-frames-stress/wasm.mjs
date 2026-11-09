import assert from "node:assert/strict";
import { readFile } from "node:fs/promises";
import { performance } from "node:perf_hooks";
import { ROOT } from "./runtime.mjs";

export async function initializeWasm() {
  const start = performance.now();
  const module = await import(
    `${ROOT}web/public/scene-wasm/opencanvas_scene_wasm.js`
  );
  module.initSync({
    module: await readFile(
      `${ROOT}web/public/scene-wasm/opencanvas_scene_wasm_bg.wasm`,
    ),
  });
  return { module, initMs: performance.now() - start };
}

/** Real wasm-bindgen module; same HTTP-authored snapshot, one mutable computer/page. */
export async function wasmPage(module, base, fixture, snapshot) {
  const computer = new module.SceneComputer(),
    encoded = JSON.stringify(snapshot);
  const start = performance.now(),
    registered = new Set(),
    rows = [];
  try {
    for (let index = 0; index < fixture.heights.length; index++) {
      const frameId = `frame-${index}`,
        height = fixture.heights[index];
      const prepareStart = performance.now();
      const fonts = JSON.parse(
        computer.required_fonts(encoded, frameId, 800, height, "default"),
      );
      const discoveryMs = performance.now() - prepareStart;
      for (const font of fonts) {
        if (registered.has(font.id)) continue;
        const response = await fetch(
          `${base}/api/v1/projects/${fixture.projectId}/fonts/${font.id}/download`,
        );
        assert.equal(response.status, 200);
        computer.load_font(
          font.id,
          font.hash,
          new Uint8Array(await response.arrayBuffer()),
        );
        registered.add(font.id);
      }
      const times = [];
      let scene;
      for (let run = 0; run < 4; run++) {
        const begun = performance.now();
        scene = JSON.parse(
          computer.compute_scene(encoded, frameId, 800, height, "default"),
        );
        times.push(performance.now() - begun);
      }
      assert.equal(scene.provenance.revision, fixture.revision);
      assert.equal(scene.nodes.length, index === 0 ? 1112 : 1111);
      assert.deepEqual(scene.provenance.viewport, { width: 800, height });
      rows.push({
        frameId,
        nodes: scene.nodes.length,
        discoveryMs,
        firstComputeMs: times[0],
        warmComputeMs: times.slice(1),
      });
    }
  } finally {
    computer.free();
  }
  assert.equal(
    rows.reduce((sum, row) => sum + row.nodes, 0),
    10000,
  );
  return {
    elapsedMs: performance.now() - start,
    registeredFontCount: registered.size,
    rows,
  };
}
