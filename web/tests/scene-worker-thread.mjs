import { parentPort } from "node:worker_threads";
import { sceneModule } from "./scene-module.mjs";

const { sceneWorkerRuntime } = await sceneModule("workerRuntime");
const handle = sceneWorkerRuntime(async () => ({
  free() {}, load_font() {}, set_resource_urls() {},
  required_fonts() { return "[]"; },
  compute_scene(snapshot) {
    // Model synchronous WASM work off the main thread, so its watchdog runs normally.
    const { delay = 0 } = JSON.parse(snapshot);
    if (delay) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, delay);
    return "{}";
  },
}), (reply) => parentPort.postMessage(reply));
parentPort.on("message", (message) => { void handle(message); });
