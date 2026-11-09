/** Observe real Worker messages and browser stalls without modifying payloads. */
export function observeBrowser() {
  const longTasks = [],
    errors = [],
    workerJobs = [],
    pending = new Map();
  window.addEventListener("error", (event) => errors.push(event.message));
  window.addEventListener("unhandledrejection", (event) =>
    errors.push(String(event.reason)),
  );
  try {
    new PerformanceObserver((list) =>
      longTasks.push(
        ...list
          .getEntries()
          .map(({ startTime, duration }) => ({ startTime, duration })),
      ),
    ).observe({ type: "longtask", buffered: true });
  } catch {}
  const OriginalWorker = window.Worker;
  window.Worker = class extends OriginalWorker {
    constructor(...args) {
      super(...args);
      this.addEventListener("message", ({ data }) => {
        const request = pending.get(data.id);
        if (!request || data.started) return;
        workerJobs.push({
          ...request,
          elapsedMs: performance.now() - request.started,
          nodes: data.result?.nodes?.length,
          error: data.error,
        });
        pending.delete(data.id);
      });
    }
    postMessage(message, ...rest) {
      pending.set(message.id, {
        kind: message.kind,
        frameId: message.request.frameId,
        projectId: message.request.projectId,
        revision: message.request.revision,
        width: message.request.width,
        height: message.request.height,
        started: performance.now(),
      });
      return super.postMessage(message, ...rest);
    }
  };
  return { longTasks, errors, workerJobs };
}
