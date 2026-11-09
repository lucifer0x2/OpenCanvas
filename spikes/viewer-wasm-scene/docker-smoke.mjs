import assert from "node:assert/strict";
import { randomUUID } from "node:crypto";
import { raw, compressedAsset } from "./docker-http.mjs";
import { connectMcp } from "../e2e/protocol.mjs";
import {
  playwrightRuntime,
  PIN,
} from "../native-render-parity/environment.mjs";
import {
  welcomeOperations,
  FRAME,
  VIEWPORT,
} from "../../samples/welcome/design.mjs";

// Run against an already running, isolated container. This script starts no service.
assert.equal(
  process.argv.length,
  3,
  "Usage: node spikes/viewer-wasm-scene/docker-smoke.mjs BASE_URL",
);
const base = new URL(process.argv[2]);
assert.ok(["http:", "https:"].includes(base.protocol));
assert.equal(base.pathname, "/", "BASE_URL must be the server origin");

const assets = {
  wasm: await compressedAsset(
    base,
    "/scene-wasm/opencanvas_scene_wasm_bg.wasm",
    /^application\/wasm(?:;|$)/,
  ),
  js: await compressedAsset(
    base,
    "/scene-wasm/opencanvas_scene_wasm.js",
    /^(?:text|application)\/javascript(?:;|$)/,
  ),
};
let browser, mcp;
try {
  mcp = await connectMcp(base.origin);
  const project = await mcp.call("create_project", {
    slug: `docker-wasm-${randomUUID()}`,
    name: "Docker WASM smoke (throwaway)",
  });
  const projectId = project.projectId;
  assert.ok(projectId);
  const applied = await mcp.call("apply_batch", {
    projectId,
    expectedRevision: 0,
    idempotencyKey: randomUUID(),
    operations: welcomeOperations(),
  });
  assert.equal(applied.committedRevision, 1);
  const projectPath = `/api/v1/projects/${encodeURIComponent(projectId)}`;
  const api = await raw(base, `${projectPath}/snapshot`, "br, gzip");
  assert.equal(api.status, 200);
  assert.equal(
    api.headers["content-encoding"],
    undefined,
    "snapshot API stays uncompressed",
  );
  assert.match(api.headers["content-type"] ?? "", /^application\/json/);
  const snapshot = JSON.parse(api.bytes);
  assert.equal(snapshot.projectId, projectId);
  assert.equal(snapshot.revision, 1);
  const sse = await raw(
    base,
    `${projectPath}/events?cursor=${encodeURIComponent(`${projectId}:1`)}`,
    "br, gzip",
    true,
  );
  assert.equal(sse.status, 200);
  assert.match(sse.headers["content-type"] ?? "", /^text\/event-stream/);
  assert.equal(
    sse.headers["content-encoding"],
    undefined,
    "SSE stays uncompressed",
  );

  const { chromium } = await playwrightRuntime();
  browser = await chromium.launch({ headless: true });
  assert.equal(browser.version(), PIN.chromium);
  const context = await browser.newContext({
    viewport: { width: 1440, height: 1000 },
  });
  const requests = [],
    pageErrors = [],
    wasmResponses = [];
  context.on("request", (request) => {
    const url = new URL(request.url());
    requests.push({ method: request.method(), path: url.pathname });
  });
  context.on("response", (response) => {
    if (
      new URL(response.url()).pathname.endsWith(
        "/opencanvas_scene_wasm_bg.wasm",
      )
    )
      wasmResponses.push({ status: response.status(), ...response.headers() });
  });
  await context.addInitScript(() => {
    window.__dockerSceneEvidence = { workers: [], scenes: [], errors: [] };
    const NativeWorker = window.Worker;
    window.Worker = class extends NativeWorker {
      constructor(...args) {
        super(...args);
        window.__dockerSceneEvidence.workers.push(String(args[0]));
        this.addEventListener("message", ({ data }) => {
          if (data.error) window.__dockerSceneEvidence.errors.push(data.error);
          if (data.result?.provenance) {
            const scene = data.result;
            window.__dockerSceneEvidence.scenes.push({
              provenance: scene.provenance,
              schemaVersion: scene.schemaVersion,
              coordinateSpace: scene.coordinateSpace,
              nodes: scene.nodes.length,
              selectedNode: scene.nodes.some((node) => node.id === "greeting"),
              text: scene.nodes
                .flatMap((node) => node.lines ?? [])
                .map((line) => line.text),
            });
          }
        });
        this.addEventListener("error", (event) =>
          window.__dockerSceneEvidence.errors.push(event.message),
        );
      }
    };
  });
  const page = await context.newPage();
  page.on("pageerror", (error) => pageErrors.push(error.message));
  const viewer = new URL("/", base);
  viewer.search = new URLSearchParams({
    scene: "wasm",
    project: projectId,
    page: "welcome-page",
    frame: FRAME,
    node: "greeting",
  });
  const navigation = await page.goto(viewer.href);
  assert.equal(navigation.status(), 200);
  await page
    .getByText("Scene r1", { exact: true })
    .waitFor({ timeout: 60_000 });
  await page.locator(".konvajs-content canvas").first().waitFor();
  await page.waitForFunction(() => {
    const canvas = document.querySelector(".konvajs-content canvas");
    return (
      canvas &&
      canvas.width > 0 &&
      canvas.height > 0 &&
      canvas
        .getContext("2d")
        .getImageData(0, 0, canvas.width, canvas.height)
        .data.some((channel, index) => index % 4 === 3 && channel > 0)
    );
  });
  const worker = await page.evaluate(() => window.__dockerSceneEvidence);
  assert.ok(
    worker.workers.some((url) => /scene\.worker/.test(url)),
    "real dedicated scene Worker",
  );
  assert.deepEqual(worker.errors, []);
  const scene = worker.scenes.findLast(
    (entry) => entry.provenance.projectId === projectId,
  );
  assert.ok(scene, "capture a computed Worker scene");
  assert.equal(scene.provenance.revision, 1);
  assert.equal(scene.provenance.frameId, FRAME);
  assert.deepEqual(scene.provenance.viewport, VIEWPORT);
  assert.equal(scene.provenance.state, "default");
  assert.ok(scene.provenance.sourceSceneKey);
  assert.equal(scene.schemaVersion, 1);
  assert.equal(scene.coordinateSpace, "frame-local");
  assert.ok(scene.nodes > 50 && scene.selectedNode);
  assert.ok(scene.text.some((text) => text.includes("Một ngày thật tốt.")));
  assert.equal(
    await page
      .getByText(/Browser scene unavailable; using server scene/)
      .count(),
    0,
  );
  assert.deepEqual(pageErrors, []);
  assert.deepEqual(
    requests.filter(({ path }) => /\/scene\/?$/.test(path)),
    [],
    "zero server scene requests",
  );
  assert.ok(requests.some(({ path }) => path === `${projectPath}/snapshot`));
  assert.ok(
    requests
      .filter(({ path }) => path.startsWith("/api/"))
      .every(({ method }) => method === "GET"),
  );
  assert.ok(wasmResponses.length > 0, "browser fetched the release module");
  for (const response of wasmResponses) {
    assert.equal(response.status, 200);
    assert.match(response["content-type"] ?? "", /^application\/wasm(?:;|$)/);
    assert.match(response["content-encoding"] ?? "", /^(br|gzip)$/);
  }
  console.log(
    JSON.stringify({
      ok: true,
      projectId,
      revision: 1,
      chromium: browser.version(),
      assets,
      apiEncoding: api.headers["content-encoding"] ?? "identity",
      sseEncoding: sse.headers["content-encoding"] ?? "identity",
      browserWasmEncoding: wasmResponses.map(
        (response) => response["content-encoding"],
      ),
      requests: requests.length,
      serverSceneRequests: 0,
      worker: {
        count: worker.workers.length,
        nodes: scene.nodes,
        provenance: scene.provenance,
      },
    }),
  );
} finally {
  await browser?.close();
  await mcp?.close();
}
