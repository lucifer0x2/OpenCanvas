import type { FontManifest, Scene } from "../../../renderer/types.ts";
import type { ComputeMessage, ComputeReply } from "./protocol.ts";
import { RevisionCache } from "./revisionCache.ts";

type Pending = {
  resolve: (result: unknown) => void;
  reject: (error: Error) => void;
  clean: () => void;
};
type Request = Omit<ComputeMessage, "id" | "kind" | "snapshot"> & {
  snapshot: NonNullable<ComputeMessage["snapshot"]>;
};

/** One lazy computer per tab; cancelled callers leave its module and font state warm. */
export class SceneWorkerClient {
  private worker?: Worker;
  private sequence = 0;
  private pending = new Map<number, Pending>();
  private snapshots = new RevisionCache<boolean>();

  constructor(
    private createWorker = () =>
      new Worker(new URL("./scene.worker.ts", import.meta.url), {
        type: "module",
      }),
    private timeoutMs = 45_000,
  ) {}

  prepare(message: Request, signal: AbortSignal): Promise<FontManifest[]> {
    return this.send({ ...message, kind: "prepare" }, signal);
  }

  compute(message: Request, signal: AbortSignal): Promise<Scene> {
    return this.send({ ...message, kind: "compute" }, signal);
  }

  private send<T>(
    message: Request & { kind: ComputeMessage["kind"] },
    signal: AbortSignal,
  ): Promise<T> {
    signal.throwIfAborted();
    return new Promise((resolve, reject) => {
      try {
        if (!this.worker) {
          this.worker = this.createWorker();
          const worker = this.worker;
          this.worker.onmessage = (event: MessageEvent<ComputeReply>) => {
            if (this.worker !== worker) return;
            const reply = event.data;
            const pending = this.pending.get(reply?.id);
            if (!pending) return;
            // A failed module import can stay cached within this worker's module map.
            if (reply.error !== undefined) {
              this.reset(new Error(reply.error));
              return;
            }
            this.pending.delete(reply.id);
            pending.clean();
            pending.resolve(reply.result);
          };
          this.worker.onerror = (event) => {
            event.preventDefault();
            if (this.worker !== worker) return;
            this.reset(new Error("Browser scene worker failed."));
          };
          this.worker.onmessageerror = () => {
            if (this.worker !== worker) return;
            this.reset(new Error("Invalid browser scene worker response."));
          };
        }
        const id = ++this.sequence;
        const abort = () => {
          const pending = this.pending.get(id);
          if (!pending) return;
          this.pending.delete(id);
          pending.clean();
          pending.reject(new Error("Browser scene request cancelled."));
        };
        const timer = setTimeout(
          () => this.reset(new Error("Browser scene computation timed out.")),
          this.timeoutMs,
        );
        const clean = () => {
          clearTimeout(timer);
          signal.removeEventListener("abort", abort);
        };
        this.pending.set(id, {
          resolve: (value) => resolve(value as T),
          reject,
          clean,
        });
        signal.addEventListener("abort", abort, { once: true });
        // Structured clone keeps cached ArrayBuffers attached for FontFace and the next request.
        const { snapshot, ...reference } = message;
        const { projectId, revision } = message.request;
        this.worker.postMessage({
          ...reference,
          id,
          ...(this.snapshots.get(projectId, revision) ? {} : { snapshot }),
        });
        this.snapshots.set(projectId, revision, true);
      } catch (error) {
        const cause =
          error instanceof Error
            ? error
            : new Error("Could not start browser scene worker.");
        this.reset(cause);
        reject(cause);
      }
    });
  }

  private reset(error: Error) {
    this.worker?.terminate();
    this.worker = undefined;
    this.snapshots.clear();
    for (const pending of this.pending.values()) {
      pending.clean();
      pending.reject(error);
    }
    this.pending.clear();
  }
}

export const sceneWorker = new SceneWorkerClient();
