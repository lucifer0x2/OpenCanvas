import type { FontManifest, Scene } from "../../../renderer/types.ts";
import type { ComputeMessage, ComputeReply } from "./protocol.ts";
import { RevisionCache } from "./revisionCache.ts";

type Pending = {
  resolve?: (result: unknown) => void;
  reject?: (error: Error) => void;
  clean: () => void;
  timer?: ReturnType<typeof setTimeout>;
  phase?: "initializing" | "computing";
  epoch: number;
  message: Request & { kind: ComputeMessage["kind"] };
};
type Request = Omit<ComputeMessage, "id" | "kind" | "snapshot"> & {
  snapshot: NonNullable<ComputeMessage["snapshot"]>;
};

/** One lazy computer per tab; cancelled callers leave its module and font state warm. */
export class SceneWorkerClient {
  private worker?: Worker;
  private sequence = 0;
  // Aborted callers retain transport records until acknowledgement or worker reset.
  private pending = new Map<number, Pending>();
  private snapshots = new RevisionCache<boolean>();
  private registered = new Map<string, string>();
  private projectId = "";
  private epoch = 0;

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

  /** Retry invalidates both ends together, including inconsistent snapshot references. */
  clearSnapshotCache() {
    this.reset(new Error("Browser scene cache invalidated."));
  }

  private receive(reply: ComputeReply) {
    const pending = this.pending.get(reply?.id);
    if ("started" in reply) {
      if (pending) this.startTimer(pending, reply.started);
      return;
    }
    if (reply.error !== undefined && !reply.errorCode) {
      // Bootstrap failures must reset even when their caller has already aborted.
      this.reset(new Error(reply.error), !pending?.reject);
      return;
    }
    if (!pending) return;
    this.pending.delete(reply.id);
    clearTimeout(pending.timer);
    pending.clean();
    if (reply.error !== undefined) {
      // A failed request may already have replaced some or all loaded fonts.
      if (pending.epoch === this.epoch) this.registered.clear();
      pending.reject?.(
        Object.assign(new Error(reply.error), { code: reply.errorCode }),
      );
      return;
    }
    if (pending.epoch === this.epoch)
      for (const font of reply.registeredFonts ?? [])
        this.registered.set(font.id, font.hash);
    pending.resolve?.(reply.result);
  }

  private startTimer(pending: Pending, phase: NonNullable<Pending["phase"]>) {
    pending.phase = phase;
    // Obsolete computation may finish at its own pace; bootstrap remains bounded.
    if (!pending.resolve && phase === "computing") {
      clearTimeout(pending.timer);
      pending.timer = undefined;
      return;
    }
    // Initialization and computation share one execution budget, excluding queue wait.
    if (pending.timer !== undefined) return;
    pending.timer = setTimeout(
      () => this.reset(new Error("Browser scene computation timed out.")),
      this.timeoutMs,
    );
  }

  private getWorker() {
    if (this.worker) return this.worker;
    const worker = (this.worker = this.createWorker());
    worker.onmessage = (event: MessageEvent<ComputeReply>) => {
      if (this.worker === worker) this.receive(event.data);
    };
    worker.onerror = (event) => {
      event.preventDefault();
      if (this.worker === worker)
        this.reset(new Error("Browser scene worker failed."));
    };
    worker.onmessageerror = () => {
      if (this.worker === worker)
        this.reset(new Error("Invalid browser scene worker response."));
    };
    return worker;
  }

  private post(id: number, pending: Pending) {
    const { snapshot, fonts, ...reference } = pending.message;
    const { projectId, revision } = pending.message.request;
    if (this.projectId !== projectId) {
      this.registered.clear();
      this.projectId = projectId;
      this.epoch++;
    }
    pending.epoch = this.epoch;
    // Earlier queued work may change font state before this request executes.
    const queued = [...this.pending.keys()].some((key) => key !== id);
    this.getWorker().postMessage({
      ...reference,
      id,
      fonts: fonts.filter(
        (font) => queued || this.registered.get(font.id) !== font.hash,
      ),
      ...(this.snapshots.get(projectId, revision) ? {} : { snapshot }),
    });
    this.snapshots.set(projectId, revision, true);
  }

  private send<T>(
    message: Request & { kind: ComputeMessage["kind"] },
    signal: AbortSignal,
  ): Promise<T> {
    signal.throwIfAborted();
    return new Promise((resolve, reject) => {
      try {
        this.getWorker();
        const id = ++this.sequence;
        const abort = () => {
          const pending = this.pending.get(id);
          if (!pending?.reject) return;
          pending.clean();
          pending.reject(new Error("Browser scene request cancelled."));
          pending.resolve = pending.reject = undefined;
          if (pending.phase === "computing") {
            clearTimeout(pending.timer);
            pending.timer = undefined;
          }
        };
        const pending: Pending = {
          resolve: (value) => resolve(value as T),
          reject,
          clean: () => signal.removeEventListener("abort", abort),
          epoch: this.epoch,
          message,
        };
        // Watch module startup; queued callers get their own timer when work starts.
        if (!this.pending.size) this.startTimer(pending, "initializing");
        this.pending.set(id, pending);
        signal.addEventListener("abort", abort, { once: true });
        this.post(id, pending);
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

  private reset(error: Error, replay = false) {
    this.worker?.terminate();
    this.worker = undefined;
    this.snapshots.clear();
    this.registered.clear();
    this.projectId = "";
    this.epoch++;
    for (const [id, pending] of this.pending) {
      clearTimeout(pending.timer);
      pending.timer = undefined;
      if (replay && pending.reject) {
        pending.phase = undefined;
        continue;
      }
      this.pending.delete(id);
      pending.clean();
      pending.reject?.(error);
    }
    if (!this.pending.size) return;
    try {
      this.startTimer(this.pending.values().next().value!, "initializing");
      for (const [id, pending] of this.pending) this.post(id, pending);
    } catch (cause) {
      this.reset(cause instanceof Error ? cause : error);
    }
  }
}

export const sceneWorker = new SceneWorkerClient();
