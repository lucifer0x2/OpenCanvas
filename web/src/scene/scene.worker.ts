import type { ComputeMessage, ComputeReply } from "./protocol";
import { sceneWorkerRuntime, type SceneComputer } from "./workerRuntime";

async function createComputer(): Promise<SceneComputer> {
  const base = `${import.meta.env.BASE_URL}scene-wasm/opencanvas_scene_wasm`;
  const wasm = await import(/* @vite-ignore */ `${base}.js`);
  await wasm.default({ module_or_path: `${base}_bg.wasm` });
  return new wasm.SceneComputer() as SceneComputer;
}

const scope = globalThis as unknown as {
  onmessage: (event: MessageEvent<ComputeMessage>) => void;
  postMessage: (reply: ComputeReply) => void;
};
const handleMessage = sceneWorkerRuntime(createComputer, (reply) =>
  scope.postMessage(reply),
);
scope.onmessage = (event) => {
  void handleMessage(event.data);
};
