import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { fixture, imageSVG, node } from "./fixtures.mjs";

let worker;
const evidence = {
  commands: "npm run test:pipeline",
  fixture: "Vietnamese nested row/column + rectangle + image",
};
before(async () => {
  worker = await PipelineWorker.start({
    resourceHandler: async (req, res) => {
      if (req.url !== "/fixture.svg") return false;
      res.writeHead(200, { "content-type": "image/svg+xml" });
      res.end(imageSVG);
      return true;
    },
  });
  evidence.browserVersion = worker.runtime.browserVersion;
  evidence.runtimeVersion = process.version;
});
after(async () => {
  await worker?.close();
  await mkdir(new URL("./output/", import.meta.url), { recursive: true });
  await writeFile(
    new URL("./output/evidence.json", import.meta.url),
    JSON.stringify(evidence, null, 2),
  );
});

test("two widths change actual line list and hug height; cached repeat has no measurement RPC", async () => {
  const input = fixture();
  const authored = JSON.stringify(input);
  const wide = await worker.compute(input);
  const narrow = await worker.compute(fixture(240));
  const wideTitle = wide.scene.nodes.find((n) => n.id === "title");
  const narrowTitle = narrow.scene.nodes.find((n) => n.id === "title");
  assert.notDeepEqual(wideTitle.lines, narrowTitle.lines);
  assert.ok(narrowTitle.lines.length > wideTitle.lines.length);
  assert.equal(wideTitle.bounds.height, wideTitle.lines.length * 32);
  assert.equal(narrowTitle.bounds.height, narrowTitle.lines.length * 32);
  assert.equal(wideTitle.bounds.width, 380);
  assert.equal(narrowTitle.bounds.width, 200);
  assert.equal(
    wide.scene.nodes.find((n) => n.id === "row").bounds.height,
    wide.scene.nodes.find((n) => n.id === "body").bounds.height + 16,
  );
  assert.ok(
    wide.metrics.measuredItems > wide.metrics.batchRPCs,
    "many misses use one batch per pass",
  );
  assert.ok(wide.metrics.layoutPasses <= 6);
  const repeat = await worker.compute(input);
  assert.equal(repeat.metrics.batchRPCs, 0);
  assert.deepEqual(repeat.scene, wide.scene);
  assert.equal(
    JSON.stringify(input),
    authored,
    "layout does not mutate authored geometry",
  );
  evidence.widths = {
    wide: { width: 420, title: wideTitle, metrics: wide.metrics },
    narrow: { width: 240, title: narrowTitle, metrics: narrow.metrics },
    repeat: repeat.metrics,
  };
  await writeFile(
    new URL("./output/wide.scene.json", import.meta.url),
    JSON.stringify(wide.scene, null, 2),
  ).catch(async () => {
    await mkdir(new URL("./output/", import.meta.url), { recursive: true });
    await writeFile(
      new URL("./output/wide.scene.json", import.meta.url),
      JSON.stringify(wide.scene, null, 2),
    );
  });
});

test("viewer canvas and PNG share scene/manifest; drawing never measures or rewraps", async () => {
  const computed = await worker.compute(fixture());
  const png = await worker.render(computed.scene);
  assert.equal(png.provenance.revision, 7);
  assert.equal(
    png.provenance.sourceSceneKey,
    computed.scene.provenance.sourceSceneKey,
  );
  assert.equal(png.width, 420);
  assert.equal(png.height, 600);
  assert.ok(
    Buffer.from(png.base64, "base64")
      .subarray(0, 8)
      .equals(Buffer.from([137, 80, 78, 71, 13, 10, 26, 10])),
  );
  const viewer = await worker.runtime.page.evaluate((scene) => {
    const measure = CanvasRenderingContext2D.prototype.measureText;
    CanvasRenderingContext2D.prototype.measureText = () => {
      throw new Error("unexpected second wrapping");
    };
    try {
      const viewerCanvas = document.createElement("canvas");
      const draw = renderer.drawScene(viewerCanvas, scene, resources);
      return {
        base64: viewerCanvas.toDataURL("image/png").split(",")[1],
        draw,
      };
    } finally {
      CanvasRenderingContext2D.prototype.measureText = measure;
    }
  }, computed.scene);
  assert.equal(
    viewer.base64,
    png.base64,
    "zero pixel tolerance on pinned runtime",
  );
  assert.equal(
    viewer.draw.drawnLines,
    computed.scene.nodes.reduce((n, x) => n + x.lines.length, 0),
  );
  await writeFile(
    new URL("./output/fixture.png", import.meta.url),
    Buffer.from(png.base64, "base64"),
  );
  evidence.render = {
    ...png,
    base64: undefined,
    comparison:
      "identical PNG bytes from separate viewer/export canvases; zero pixel tolerance",
  };
});

test("strict readiness waits for delayed real image decode", async () => {
  await worker.runtime.page.evaluate(() => {
    globalThis.originalDecode = HTMLImageElement.prototype.decode;
    HTMLImageElement.prototype.decode = async function () {
      await originalDecode.call(this);
      await new Promise((resolve) => setTimeout(resolve, 180));
      globalThis.decodeFinished = true;
    };
    globalThis.decodeFinished = false;
  });
  try {
    const start = performance.now();
    const result = await worker.compute(fixture());
    assert.ok(performance.now() - start >= 180);
    assert.equal(
      await worker.runtime.page.evaluate(() => decodeFinished),
      true,
    );
    assert.equal(
      result.readiness.find((r) => r.kind === "asset").status,
      "ready",
    );
    evidence.delayedImageDecode = {
      minimumDelayMs: 180,
      observedMs: performance.now() - start,
    };
  } finally {
    await worker.runtime.page.evaluate(() => {
      HTMLImageElement.prototype.decode = originalDecode;
    });
  }
});

test("missing font, wrong font hash and missing image never report strict success", async () => {
  const missing = fixture();
  missing.fonts[0].url = "/missing.ttf";
  await assert.rejects(
    worker.compute(missing),
    (error) => error.code === "RESOURCE_NOT_READY",
  );
  const badHash = fixture();
  badHash.fonts[0].hash = "0".repeat(64);
  await assert.rejects(
    worker.compute(badHash),
    (error) => error.code === "RESOURCE_NOT_READY",
  );
  const image = fixture();
  image.assets[0].url = "/missing.svg";
  await assert.rejects(
    worker.compute(image),
    (error) => error.code === "RESOURCE_NOT_READY",
  );
  const placeholder = await worker.compute(image, { strict: false });
  assert.ok(placeholder.scene.diagnostics.length > 0);
  const relaxedImage = await worker.render(placeholder.scene, {
    strict: false,
  });
  assert.equal(
    relaxedImage.readiness.find((r) => r.kind === "asset").status,
    "failed",
  );
  await assert.rejects(
    worker.render(placeholder.scene),
    (error) => error.code === "RESOURCE_NOT_READY",
  );
  const scene = (await worker.compute(fixture())).scene;
  scene.fonts[0].url = "/missing.ttf";
  await assert.rejects(
    worker.render(scene),
    (error) => error.code === "RESOURCE_NOT_READY",
  );
  const relaxed = await worker.render(scene, { strict: false });
  assert.equal(
    relaxed.readiness.find((r) => r.kind === "font").status,
    "failed",
  );
  assert.ok(relaxed.diagnostics.length > 0);
  evidence.readinessFailures = [
    "missing-font",
    "font-hash-mismatch",
    "missing-image",
    "strict-export-missing-font",
  ];
});

test("bounded relayout and pixel limits fail explicitly", async () => {
  const fresh = fixture(237);
  fresh.nodes.find((n) => n.id === "title").props.text += " Uncached.";
  await assert.rejects(
    worker.compute(fresh, { maxPasses: 1 }),
    (error) => error.code === "LAYOUT_NON_CONVERGENCE",
  );
  const scene = (await worker.compute(fixture())).scene;
  await assert.rejects(
    worker.render(scene, { scale: 5 }),
    (error) => error.code === "PIXEL_BUDGET",
  );
  evidence.limits = { passes: 6, pixels: 16000000, scale: 4, concurrency: 1 };
});
