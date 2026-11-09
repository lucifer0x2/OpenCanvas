import { sceneModule } from "./scene-module.mjs";

export const { SceneWorkerClient } = await sceneModule("workerClient");
export const { sceneWorkerRuntime } = await sceneModule("workerRuntime");
export const signal = () => new AbortController().signal;
export const message = (revision = 1, projectId = "p") => ({
  request: { projectId, revision, frameId: "frame", width: 800, height: 600 },
  snapshot: { projectId, revision, nodes: [] },
  fonts: [{ id: "font", hash: "hash", bytes: new Uint8Array([1]).buffer }],
  urls: { fonts: {}, assets: {} },
});
export const computer = (overrides = {}) => ({
  free() {}, load_font() {}, set_resource_urls() {},
  required_fonts() { return "[]"; },
  compute_scene() { return "{}"; },
  ...overrides,
});

export class WorkerStub {
  messages = [];
  terminated = false;
  postMessage(message) { this.messages.push(structuredClone(message)); }
  terminate() { this.terminated = true; }
  reply(reply) { this.onmessage({ data: reply }); }
}

export function harness(createComputer, timeout = 1000) {
  const workers = [];
  const client = new SceneWorkerClient(() => {
    const worker = new WorkerStub();
    const handle = sceneWorkerRuntime(createComputer, (reply) => worker.reply(reply));
    worker.postMessage = function (message) {
      this.messages.push(structuredClone(message));
      void handle(structuredClone(message));
    };
    workers.push(worker);
    return worker;
  }, timeout);
  return { client, workers };
}

export function deferred() {
  let resolve, reject;
  const promise = new Promise((yes, no) => { resolve = yes; reject = no; });
  return { promise, resolve, reject };
}
