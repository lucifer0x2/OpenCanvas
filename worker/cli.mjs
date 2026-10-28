import { createInterface } from "node:readline";
import { PipelineWorker } from "./pipeline.mjs";

// NDJSON in/out. Diagnostics are structured and contain no local filesystem paths.
let worker;
try {
  worker = await PipelineWorker.start();
  for await (const line of createInterface({
    input: process.stdin,
    crlfDelay: Infinity,
  })) {
    let request;
    try {
      request = JSON.parse(line);
      let result;
      if (request.op === "compute")
        result = await worker.compute(request.input, {
          strict: request.strict !== false,
        });
      else if (request.op === "render")
        result = await worker.render(request.scene, {
          scale: request.scale ?? 1,
          strict: request.strict !== false,
        });
      else if (request.op === "compute_render") {
        const computed = await worker.compute(request.input);
        result = {
          ...computed,
          render: await worker.render(computed.scene, {
            scale: request.scale ?? 1,
          }),
        };
      } else
        throw Object.assign(new Error("Unknown worker operation"), {
          code: "INVALID_OPERATION",
        });
      process.stdout.write(
        JSON.stringify({ id: request.id ?? null, ...result }) + "\n",
      );
    } catch (error) {
      process.stdout.write(
        JSON.stringify({
          id: request?.id ?? null,
          status: "failed",
          code: error.code ?? "WORKER_FAILED",
          message: error.code ? error.message : "Worker request failed",
        }) + "\n",
      );
    }
  }
} catch {
  process.stdout.write(
    JSON.stringify({
      status: "failed",
      code: "WORKER_UNAVAILABLE",
      message: "Chromium worker could not start",
    }) + "\n",
  );
  process.exitCode = 1;
} finally {
  await worker?.close();
}
