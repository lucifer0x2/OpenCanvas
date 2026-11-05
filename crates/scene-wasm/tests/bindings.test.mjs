// Explicit generated-artifact test: run after npm run build:scene-wasm.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { readFileSync } from "node:fs";
import test from "node:test";
import init, { SceneComputer } from "../../../web/public/scene-wasm/opencanvas_scene_wasm.js";

await init({ module_or_path: readFileSync(new URL(
  "../../../web/public/scene-wasm/opencanvas_scene_wasm_bg.wasm", import.meta.url,
)) });
const bytes = readFileSync(new URL("../../../renderer/fonts/NotoSans.ttf", import.meta.url));
const hash = createHash("sha256").update(bytes).digest("hex");

function snapshot() {
  return {
    schemaVersion: 4, projectId: "project", slug: "test", name: "Test", revision: 7,
    nodes: [
      { id: "page", parentId: null, sortKey: 0, kind: "page", props: {} },
      { id: "frame", parentId: "page", sortKey: 0, kind: "frame",
        props: { width: 200, height: 120, layout: { mode: "column" } } },
      { id: "text", parentId: "frame", sortKey: 0, kind: "text",
        props: { text: "Tiếng Việt — OpenCanvas wraps exactly", fontId: "font",
          fontSize: 16, lineHeight: 24, width: "fill", height: "hug" } },
    ],
    tokens: [], styles: [], assets: [],
    fonts: [{ id: "font", family: "Noto Sans", weight: 400, style: "normal",
      assetId: null, bundledId: "noto", contentHash: hash, license: "OFL", status: "ready" }],
    retention: { oldestCommitRevision: 0, idempotencyPolicy: "bounded" },
  };
}

test("generated ESM exposes exact-font scene API with structured errors", () => {
  const computer = new SceneComputer();
  const json = JSON.stringify(snapshot());
  const required = JSON.parse(computer.required_fonts(json, "frame", 200, 120, "default"));
  assert.equal(required[0].hash, hash);
  assert.throws(() => computer.compute_scene(json, "frame", 200, 120, "default"),
    (error) => error.code === "FONT_MISSING" && error.retryable === false);
  computer.load_font("font", hash, bytes);
  computer.set_resource_urls(JSON.stringify({ fonts: { font: "/caller/font" }, assets: {} }));
  const wide = JSON.parse(computer.compute_scene(json, "frame", 200, 120, "default"));
  const narrow = JSON.parse(computer.compute_scene(json, "frame", 80, 120, "default"));
  assert.equal(wide.fonts[0].url, "/caller/font");
  assert.equal(wide.provenance.sceneVersion, "scene-3");
  assert.equal(wide.provenance.rendererVersion, "web-4");
  assert.ok(narrow.nodes[1].lines.length > wide.nodes[1].lines.length);
  assert.throws(() => computer.load_font("font", hash, new Uint8Array([1])),
    (error) => error.code === "RESOURCE_HASH_MISMATCH");
  assert.deepEqual(JSON.parse(computer.compute_scene(json, "frame", 200, 120, "default")), wide);
  computer.free();
});

test("generated discovery and compute reject viewport budgets before resolving or loading fonts", () => {
  const computer = new SceneComputer();
  const json = JSON.stringify(snapshot());
  for (const [width, height] of [
    [NaN, 1], [Infinity, 1], [1, -Infinity], [0, 1], [-1, 1],
    [1, 0], [1, -1], [100001, 1], [1, 100001], [4000, 4000.001], [8000, 8000],
  ]) {
    for (const method of ["required_fonts", "compute_scene"]) {
      assert.throws(() => computer[method](json, "frame", width, height, "absent"),
        (error) => error.code === "INVALID_REQUEST" && error.retryable === false &&
          error.message === "Viewport or scale exceeds the render budget");
    }
  }
  const empty = snapshot();
  empty.nodes.pop();
  const nontext = JSON.stringify(empty);
  for (const [width, height] of [[4000, 4000], [100000, 160]]) {
    assert.deepEqual(JSON.parse(computer.required_fonts(nontext, "frame", width, height, "default")), []);
    const scene = JSON.parse(computer.compute_scene(nontext, "frame", width, height, "default"));
    assert.deepEqual(scene.provenance.viewport, { width, height });
  }
  computer.free();
});
