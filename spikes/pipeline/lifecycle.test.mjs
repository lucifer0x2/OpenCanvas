import { test } from "node:test";
import assert from "node:assert/strict";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { fixture, imageSVG } from "./fixtures.mjs";

test("resource decode timeout produces explicit failed readiness", async () => {
  const worker = await PipelineWorker.start({
    resourceHandler: async (req, res) => {
      if (req.url !== "/fixture.svg") return false;
      res.writeHead(200, { "content-type": "image/svg+xml" });
      res.end(imageSVG);
      return true;
    },
  });
  try {
    const statuses = await worker.runtime.page.evaluate(async (assets) => {
      const original = HTMLImageElement.prototype.decode;
      HTMLImageElement.prototype.decode = () => new Promise(() => {});
      try {
        return (
          await renderer.loadResources([], assets, {
            strict: false,
            timeoutMs: 40,
          })
        ).readiness;
      } finally {
        HTMLImageElement.prototype.decode = original;
      }
    }, fixture().assets);
    assert.deepEqual(statuses, [
      {
        id: "image",
        kind: "asset",
        status: "failed",
        code: "RESOURCE_TIMEOUT",
      },
    ]);
  } finally {
    await worker.close();
  }
});

test("one job at a time; explicit cancellation stops active job and requires worker restart", async () => {
  let entered;
  const started = new Promise((resolve) => {
    entered = resolve;
  });
  const worker = await PipelineWorker.start({
    resourceHandler: async (req, res) => {
      if (req.url !== "/slow-font.ttf") return false;
      entered();
      req.once("close", () => res.end());
      return true;
    },
  });
  try {
    const input = fixture();
    input.fonts[0].url = "/slow-font.ttf";
    input.assets = [];
    const active = worker.compute(input);
    // Attach the rejection observer before cancellation to avoid unhandled-rejection timing.
    const cancelled = assert.rejects(
      active,
      (error) => error.code === "JOB_CANCELLED",
    );
    await started;
    await assert.rejects(
      worker.compute(input),
      (error) => error.code === "WORKER_BUSY",
    );
    worker.cancel();
    await cancelled;
    await assert.rejects(
      worker.compute(input),
      (error) => error.code === "WORKER_UNAVAILABLE",
    );
  } finally {
    await worker.close();
  }
});
