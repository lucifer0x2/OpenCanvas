import type { FontManifest, Scene } from "../../../renderer/types";
import type { ComputeMessage, ComputeReply } from "./protocol";
import { RevisionCache } from "./revisionCache";

export interface SceneComputer {
  free(): void;
  load_font(id: string, hash: string, bytes: Uint8Array): void;
  set_resource_urls(urlsJSON: string): void;
  required_fonts(
    snapshot: string,
    frame: string,
    width: number,
    height: number,
    theme: string,
  ): string;
  compute_scene(
    snapshot: string,
    frame: string,
    width: number,
    height: number,
    theme: string,
  ): string;
}

/** The serial queue owns snapshot encoding, WASM initialization and mutable font state. */
export function sceneWorkerRuntime(
  createComputer: () => Promise<SceneComputer>,
  reply: (message: ComputeReply) => void,
) {
  let initializing: Promise<SceneComputer> | undefined;
  let projectId = "";
  const registered = new Map<string, string>();
  const snapshots = new RevisionCache<string>();
  let queue = Promise.resolve();

  async function compute(
    message: ComputeMessage,
  ): Promise<Scene | FontManifest[]> {
    if (projectId && projectId !== message.request.projectId) {
      (await initializing)?.free();
      initializing = undefined;
      registered.clear();
    }
    projectId = message.request.projectId;
    const { revision, frameId, width, height } = message.request;
    let snapshotJSON = snapshots.get(projectId, revision);
    if (snapshotJSON === undefined) {
      if (
        message.snapshot?.projectId !== projectId ||
        message.snapshot.revision !== revision
      )
        throw new Error(
          "Browser scene snapshot cache is missing this revision.",
        );
      snapshotJSON = JSON.stringify(message.snapshot);
      snapshots.set(projectId, revision, snapshotJSON);
    }
    const instance = await (initializing ??= createComputer());
    instance.set_resource_urls(JSON.stringify(message.urls));
    if (message.kind === "prepare")
      return JSON.parse(
        instance.required_fonts(
          snapshotJSON,
          frameId,
          width,
          height,
          "default",
        ),
      );
    for (const font of message.fonts) {
      if (registered.get(font.id) === font.hash) continue;
      instance.load_font(font.id, font.hash, new Uint8Array(font.bytes));
      registered.set(font.id, font.hash);
    }
    return JSON.parse(
      instance.compute_scene(snapshotJSON, frameId, width, height, "default"),
    );
  }

  return (message: ComputeMessage): Promise<void> => {
    // Overlap is permitted by the client; mutable WASM state stays sequential here.
    queue = queue.then(async () => {
      try {
        reply({ id: message.id, result: await compute(message) });
      } catch (cause) {
        const error = cause as { message?: string };
        reply({
          id: message.id,
          error: error?.message ?? "Browser scene computation failed.",
        });
      }
    });
    return queue;
  };
}
