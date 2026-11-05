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

test("scene engine defaults to WASM and honors only recognized preferences", () => {
  for (const saved of [null, "wasm", "", "unknown"])
    assert.equal(wasmSceneEnabled("", { getItem: () => saved }), true);
  assert.equal(wasmSceneEnabled("", { getItem: () => "server" }), false);
  for (const selected of ["", "unknown"])
    assert.equal(
      wasmSceneEnabled(`?scene=${selected}`, { getItem: () => null }),
      true,
      "unknown query values keep the WASM default",
    );
  assert.equal(
    wasmSceneEnabled("?scene=unknown", { getItem: () => "server" }),
    false,
    "unknown query values preserve an explicit saved preference",
  );
});

test("recognized scene query overrides storage without reading it", () => {
  for (const selected of ["wasm", "server"])
    for (const saved of ["wasm", "server"])
      assert.equal(
        wasmSceneEnabled(`?scene=${selected}`, { getItem: () => saved }),
        selected === "wasm",
      );
  for (const selected of ["wasm", "server"])
    assert.equal(
      wasmSceneEnabled(`?scene=${selected}`, {
        getItem: () => {
          throw Error("must not read storage");
        },
      }),
      selected === "wasm",
    );
});

test("unavailable or denied localStorage keeps the WASM default", () => {
  assert.equal(
    wasmSceneEnabled("", {
      getItem: () => {
        throw Error("denied");
      },
    }),
    true,
  );
  const previous = Object.getOwnPropertyDescriptor(globalThis, "localStorage");
  try {
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      value: undefined,
    });
    assert.equal(wasmSceneEnabled(""), true);
    Object.defineProperty(globalThis, "localStorage", {
      configurable: true,
      get: () => {
        throw Error("denied");
      },
    });
    assert.equal(wasmSceneEnabled(""), true);
  } finally {
    if (previous) Object.defineProperty(globalThis, "localStorage", previous);
    else delete globalThis.localStorage;
  }
});

test("explicit server selection skips browser compute; browser success never calls server", async () => {
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

test("browser prepare budget rejection falls back and propagates the server INVALID_REQUEST without a scene", async () => {
  const previousFetch = globalThis.fetch;
  const budget = {
    code: "INVALID_REQUEST",
    message: "Viewport or scale exceeds the render budget",
    retryable: false,
  };
  let prepares = 0;
  let serverCalls = 0;
  const rejectedPreparation = async () => {
    prepares++;
    throw budget;
  };
  globalThis.fetch = async (url) => {
    assert.match(
      url,
      /\/scene\?/,
      "no font or asset fetch after prepare rejection",
    );
    serverCalls++;
    return new Response(JSON.stringify({ error: budget }), {
      status: 400,
      headers: { "Content-Type": "application/json" },
    });
  };
  try {
    const overBudget = { ...request, width: 4000, height: 4000.001 };
    await assert.rejects(
      loadScene(overBudget, snapshot, signal(), true, rejectedPreparation),
      (error) =>
        error.code === "INVALID_REQUEST" &&
        error.message === budget.message &&
        error.status === 400,
    );
    assert.equal(prepares, 1);
    assert.equal(serverCalls, 1);
  } finally {
    globalThis.fetch = previousFetch;
  }
});
