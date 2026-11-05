import assert from "node:assert/strict";
import test from "node:test";
import { sceneModule } from "./scene-module.mjs";

const { loadScene, WASM_FALLBACK } = await sceneModule("service");
const { snapshotResources, decorateResources } = await sceneModule("manifests");
const { wasmSceneEnabled } = await sceneModule("flag");
const request = {
  projectId: "a/b",
  revision: 8,
  frameId: "frame",
  width: 800,
  height: 600,
};
const snapshot = {
  ...request,
  nodes: [],
  tokens: [],
  styles: [],
  truncated: false,
  fonts: [
    {
      id: "f/1",
      contentHash: "a".repeat(64),
      family: "Font",
      weight: 400,
      style: "normal",
    },
  ],
  assets: [{ id: "a/1", contentHash: "b".repeat(64), mediaType: "image/png" }],
};
const signal = () => new AbortController().signal;

test("scene engine is server by default and explicit query overrides storage", () => {
  assert.equal(wasmSceneEnabled("", { getItem: () => null }), false);
  assert.equal(wasmSceneEnabled("?scene=wasm"), true);
  assert.equal(
    wasmSceneEnabled("?scene=server", { getItem: () => "wasm" }),
    false,
  );
  assert.equal(wasmSceneEnabled("", { getItem: () => "wasm" }), true);
  assert.equal(
    wasmSceneEnabled("", {
      getItem: () => {
        throw Error("denied");
      },
    }),
    false,
  );
});

test("server default never starts browser compute; browser success never calls server", async () => {
  let called = 0;
  const server = async () => ({ status: "ready", diagnostics: [] });
  const browser = async (desired, accepted) => {
    called++;
    assert.equal(desired, request);
    assert.equal(accepted, snapshot);
    return { job: { status: "ready", jobId: "browser" } };
  };
  assert.equal(
    (await loadScene(request, snapshot, signal(), false, browser, server)).job
      .status,
    "ready",
  );
  assert.equal(called, 0);
  const result = await loadScene(
    request,
    snapshot,
    signal(),
    true,
    browser,
    () => {
      throw Error("unexpected server");
    },
  );
  assert.equal(result.job.jobId, "browser");
  assert.equal(called, 1);
});

test("unavailable browser falls back once with diagnostic and cancelled work never falls back", async () => {
  let serverCalls = 0;
  const server = async () => {
    serverCalls++;
    return { diagnostics: ["server detail"] };
  };
  const browser = async () => {
    throw Error("bad wasm");
  };
  const result = await loadScene(
    request,
    snapshot,
    signal(),
    true,
    browser,
    server,
  );
  assert.deepEqual(result.job.diagnostics, [
    "server detail",
    `${WASM_FALLBACK}: bad wasm`,
  ]);
  const cancelled = new AbortController();
  const aborting = async () => {
    cancelled.abort();
    throw Error("obsolete");
  };
  await assert.rejects(
    loadScene(request, snapshot, cancelled.signal, true, aborting, server),
    /aborted/,
  );
  assert.equal(serverCalls, 1);
});

test("snapshot adapter rejects stale and truncated input, confines all resource URLs", () => {
  const manifests = snapshotResources(snapshot, request);
  assert.equal(
    manifests.fonts[0].url,
    "/api/v1/projects/a%2Fb/fonts/f%2F1/download",
  );
  assert.equal(manifests.assets[0].url, "/api/v1/projects/a%2Fb/assets/a%2F1");
  for (const patch of [
    { revision: 7 },
    { projectId: "foreign" },
    { truncated: true },
    { tokens: undefined },
  ])
    assert.throws(
      () => snapshotResources({ ...snapshot, ...patch }, request),
      /snapshot|collections/,
    );
  const scene = {
    fonts: [{ ...manifests.fonts[0], url: "https://untrusted.invalid/font" }],
    assets: manifests.assets,
  };
  assert.equal(
    decorateResources(scene, manifests).fonts[0].url,
    manifests.fonts[0].url,
  );
  assert.throws(
    () =>
      decorateResources(
        { ...scene, fonts: [{ ...scene.fonts[0], hash: "different" }] },
        manifests,
      ),
    /manifest/,
  );
});
