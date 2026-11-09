/** Give paint/input a turn, and avoid admitting background-tab compute work. */
export function yieldFrame(signal: AbortSignal): Promise<void> {
  signal.throwIfAborted();
  return new Promise((resolve, reject) => {
    let raf = 0;
    let timer: ReturnType<typeof setTimeout>;
    const cleanup = () => {
      clearTimeout(timer);
      if (typeof cancelAnimationFrame === "function") cancelAnimationFrame(raf);
      globalThis.document?.removeEventListener("visibilitychange", visible);
      signal.removeEventListener("abort", abort);
    };
    const finish = () => {
      cleanup();
      resolve();
    };
    const abort = () => {
      cleanup();
      reject(signal.reason);
    };
    const visible = () => {
      if (globalThis.document?.hidden) return;
      cleanup();
      signal.addEventListener("abort", abort, { once: true });
      // A macrotask after rAF lets the previous frame's React/Konva update paint.
      if (typeof requestAnimationFrame === "function")
        raf = requestAnimationFrame(() => {
          timer = setTimeout(finish, 0);
        });
      else timer = setTimeout(finish, 0);
    };
    signal.addEventListener("abort", abort, { once: true });
    if (globalThis.document?.hidden)
      document.addEventListener("visibilitychange", visible);
    else visible();
  });
}
