import { spawnSync } from "node:child_process";
import path from "node:path";
import { root, startRuntime } from "./runtime.mjs";

export const BUNDLED_FONT = Object.freeze({
  id: "font_bundled",
  family: "OpenCanvas Noto Sans",
  hash: "bfb7bb691513f12e734dc346c03a03f784912432d7e3fa8e56efcf906fe86b3d",
  url: "/renderer/fonts/NotoSans.ttf",
  weight: 400,
  style: "normal",
});
const MAX_PASSES = 6;
const MAX_CACHE = 10_000;
export class WorkerError extends Error {
  constructor(code, message) {
    super(message);
    this.code = code;
  }
}

/** Single job at a time, finite passes/time/cache/pixels. Database transactions stay outside. */
export class PipelineWorker {
  static async start(options) {
    return new PipelineWorker(await startRuntime(options));
  }
  constructor(runtime) {
    this.runtime = runtime;
    this.cache = {};
    this.busy = false;
  }
  async close() {
    await this.cancelClose;
    await this.runtime.close();
  }
  async exclusive(fn) {
    if (this.terminal)
      throw new WorkerError(
        "WORKER_UNAVAILABLE",
        "Worker must restart after cancellation or timeout",
      );
    if (this.busy)
      throw new WorkerError("WORKER_BUSY", "Worker concurrency limit is 1");
    this.busy = true;
    let timer;
    const cancelled = new Promise((_, reject) => {
      this.abort = reject;
      timer = setTimeout(() => this.cancel("JOB_TIMEOUT"), 30_000);
    });
    try {
      return await Promise.race([fn(), cancelled]);
    } finally {
      clearTimeout(timer);
      this.abort = undefined;
      this.busy = false;
    }
  }
  cancel(code = "JOB_CANCELLED") {
    if (!this.busy) return;
    this.terminal = true;
    this.abort?.(
      new WorkerError(
        code,
        code === "JOB_TIMEOUT" ? "Job exceeded 30 seconds" : "Job cancelled",
      ),
    );
    this.cancelClose = this.runtime.page.close().catch(() => {});
  }
  async resources(fonts, assets, strict = true) {
    return this.runtime.page
      .evaluate(
        async ({ fonts, assets, strict }) => {
          try {
            globalThis.resources = await renderer.loadResources(fonts, assets, {
              strict,
              timeoutMs: 10_000,
            });
            return {
              readiness: resources.readiness,
              diagnostics: resources.diagnostics,
            };
          } catch (error) {
            return {
              error: {
                code: error.code ?? "RESOURCE_NOT_READY",
                message: error.message,
              },
            };
          }
        },
        { fonts, assets, strict },
      )
      .then((result) => {
        if (result.error)
          throw new WorkerError(result.error.code, result.error.message);
        return result;
      });
  }
  layoutStep(input, cache) {
    const binary =
      process.env.OPENCANVAS_LAYOUT_BIN ??
      path.join(root, "target/debug/layout-step");
    const result = spawnSync(binary, {
      input: JSON.stringify({ input, cache }) + "\n",
      encoding: "utf8",
      timeout: 5_000,
      maxBuffer: 16 * 1024 * 1024,
    });
    if (result.error || result.status !== 0)
      throw new WorkerError(
        "LAYOUT_WORKER_FAILED",
        "Rust layout worker unavailable or timed out",
      );
    let step;
    try {
      step = JSON.parse(result.stdout);
    } catch {
      throw new WorkerError(
        "LAYOUT_WORKER_FAILED",
        "Invalid Rust layout response",
      );
    }
    if (step.status === "failed")
      throw new WorkerError(step.code, step.message);
    return step;
  }
  async compute(input, { maxPasses = MAX_PASSES, strict = true } = {}) {
    return this.exclusive(async () => {
      const start = performance.now();
      const resources = await this.resources(input.fonts, input.assets, strict);
      if (Object.keys(this.cache).length > MAX_CACHE) this.cache = {};
      const metrics = {
        layoutPasses: 0,
        batchRPCs: 0,
        measuredItems: 0,
        cacheEntriesBefore: Object.keys(this.cache).length,
      };
      for (let pass = 0; pass < Math.min(MAX_PASSES, maxPasses); pass++) {
        if (performance.now() - start > 30_000)
          throw new WorkerError("JOB_TIMEOUT", "Compute exceeded 30 seconds");
        metrics.layoutPasses++;
        const step = this.layoutStep(input, this.cache);
        if (step.status === "ready") {
          step.scene.diagnostics.push(...resources.diagnostics);
          return {
            status: "ready",
            scene: step.scene,
            metrics: { ...metrics, elapsedMs: performance.now() - start },
            ...resources,
          };
        }
        if (step.status !== "pending" || !step.requests.length)
          throw new WorkerError(
            "LAYOUT_WORKER_FAILED",
            "Invalid measurement batch",
          );
        metrics.batchRPCs++;
        metrics.measuredItems += step.requests.length;
        const measured = await this.runtime.page.evaluate((requests) => {
          try {
            return { cache: renderer.measureBatch(requests, resources) };
          } catch (error) {
            return {
              error: {
                code: error.code ?? "MEASURE_FAILED",
                message: error.message,
              },
            };
          }
        }, step.requests);
        if (measured.error)
          throw new WorkerError(measured.error.code, measured.error.message);
        Object.assign(this.cache, measured.cache);
      }
      throw new WorkerError(
        "LAYOUT_NON_CONVERGENCE",
        `Layout did not converge within ${Math.min(MAX_PASSES, maxPasses)} passes`,
      );
    });
  }
  async render(scene, { scale = 1, strict = true } = {}) {
    return this.exclusive(async () => {
      const resources = await this.resources(scene.fonts, scene.assets, strict);
      const result = await this.runtime.page.evaluate(
        ({ scene, scale, strict }) => {
          try {
            const canvas = document.getElementById("scene");
            const draw = renderer.drawScene(canvas, scene, resources, {
              scale,
              strict,
            });
            return {
              base64: canvas.toDataURL("image/png").split(",")[1],
              width: canvas.width,
              height: canvas.height,
              draw,
            };
          } catch (error) {
            return {
              error: {
                code: error.code ?? "RENDER_FAILED",
                message: error.message,
              },
            };
          }
        },
        { scene, scale, strict },
      );
      if (result.error)
        throw new WorkerError(result.error.code, result.error.message);
      return {
        status: "ready",
        mimeType: "image/png",
        ...result,
        ...resources,
        provenance: {
          ...scene.provenance,
          scale,
          browserVersion: this.runtime.browserVersion,
          runtimeVersion: process.version,
          readiness: resources.readiness,
        },
      };
    });
  }
}
