import assert from "node:assert/strict";
import { fileURLToPath } from "node:url";

export async function openViewer(browser, base) {
  const page = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
    deviceScaleFactor: 1,
  });
  const requests = [];
  const errors = [];
  const scenes = [];
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("request", (request) => {
    if (new URL(request.url()).pathname.startsWith("/api/"))
      requests.push({
        method: request.method(),
        path: new URL(request.url()).pathname,
      });
  });
  page.on("response", async (response) => {
    if (/\/scene\?|\/jobs\/[^/]+$/.test(response.url())) {
      try {
        const value = await response.json();
        if (value.scene) scenes.push(value.scene);
      } catch {}
    }
  });
  // Observe the real canvases the viewer rasterizes with the shared renderer, keyed by size, so
  // the 1:1 one can be compared with the worker PNG while the viewer also draws the scene at its
  // display scale. A canvas that has itself received a canvas copy is a derived target -- a Konva
  // layer, or a zoom-out level -- and is skipped. This adds no app state or alternate renderer
  // and leaves every draw unchanged.
  await page.addInitScript(() => {
    const original = CanvasRenderingContext2D.prototype.drawImage;
    const derived = new WeakSet();
    globalThis.__liveCanvases = {};
    CanvasRenderingContext2D.prototype.drawImage = function (image, ...args) {
      if (image instanceof HTMLCanvasElement) {
        if (image !== this.canvas && image.width > 1 && !derived.has(image)) {
          try {
            globalThis.__liveCanvases[`${image.width}x${image.height}`] =
              image.toDataURL("image/png");
          } catch {}
        }
        derived.add(this.canvas);
      }
      return original.call(this, image, ...args);
    };
  });
  await page.goto(base);
  return { page, requests, errors, scenes };
}

export const ready = (page, revision) =>
  page
    .getByText(`Scene r${revision}`, { exact: true })
    .waitFor({ timeout: 50_000 });

export async function compareViewer(view, job) {
  const { page, scenes } = view;
  await ready(page, job.revision);
  const { width: vw, height: vh } = job.scene.provenance.viewport;
  const key = `${vw}x${vh}`;
  await page.waitForFunction(
    (size) => Boolean(globalThis.__liveCanvases[size]),
    key,
  );
  const scene = scenes.findLast(
    (candidate) => candidate.provenance.sourceSceneKey === job.sceneKey,
  );
  assert.ok(scene, "viewer must consume the same backend scene key");
  assert.deepEqual(
    scene,
    job.scene,
    "viewer and MCP scene/lines/provenance match exactly",
  );
  const pixels = await page.evaluate(
    async ({ url, key, width, height }) => {
      const decode = async (source) => {
        const image = new Image();
        image.src = source;
        await image.decode();
        const canvas = document.createElement("canvas");
        canvas.width = image.width;
        canvas.height = image.height;
        canvas.getContext("2d").drawImage(image, 0, 0);
        return {
          width: image.width,
          height: image.height,
          bytes: canvas
            .getContext("2d")
            .getImageData(0, 0, image.width, image.height).data,
        };
      };
      const actual = await decode(globalThis.__liveCanvases[key]);
      const expected = await decode(url);
      if (actual.width !== width || actual.height !== height)
        throw new Error(`Viewer source size ${actual.width}x${actual.height}`);
      let differences = 0;
      for (let i = 0; i < actual.bytes.length; i++)
        if (actual.bytes[i] !== expected.bytes[i]) differences++;
      return {
        width: expected.width,
        height: expected.height,
        differences,
        comparedChannels: actual.bytes.length,
      };
    },
    { url: job.imageUrl, key, ...job.scene.provenance.viewport },
  );
  assert.equal(pixels.differences, 0, "exact RGBA equality, no tolerance");
  assert.equal(pixels.width, job.image.width);
  assert.equal(pixels.height, job.image.height);
  return pixels;
}

export async function interact(view, artifacts) {
  const { page } = view;
  await page.getByRole("button", { name: "Headline", exact: true }).click();
  await page.getByRole("heading", { name: "Headline", exact: true }).waitFor();
  const before = await page.getByLabel("Zoom level").textContent();
  await page.getByRole("button", { name: "Zoom in", exact: true }).click();
  assert.notEqual(await page.getByLabel("Zoom level").textContent(), before);
  const canvas = page.getByLabel(/^Canvas\. Drag/);
  await canvas.focus();
  const beforePan = await page
    .locator(".konvajs-content canvas")
    .first()
    .evaluate((node) => node.toDataURL());
  await page.keyboard.press("ArrowRight");
  await page.waitForFunction(
    (before) =>
      document.querySelector(".konvajs-content canvas")?.toDataURL() !== before,
    beforePan,
  );
  await page.keyboard.press("f");
  await page.getByLabel("Zoom level").filter({ hasText: before }).waitFor();
  await page.screenshot({
    path: fileURLToPath(new URL("desktop.png", artifacts)),
  });
}

export function assertReadOnly(view) {
  assert.ok(view.requests.length > 0);
  assert.ok(
    view.requests.every((entry) => entry.method === "GET"),
    JSON.stringify(view.requests),
  );
  assert.deepEqual(view.errors, []);
}
