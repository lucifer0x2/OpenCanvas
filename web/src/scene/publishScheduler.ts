type Timer = ReturnType<typeof setTimeout>;

/** Timing sources, injectable for tests; defaults are the browser's. */
export interface FrameClock {
  requestFrame?: (run: () => void) => number;
  cancelFrame?: (id: number) => void;
  setTimer: (run: () => void, ms: number) => Timer;
  clearTimer: (id: Timer) => void;
}

export interface PublishScheduler {
  /** Commit at the next animation frame; repeated calls before it share one commit. */
  schedule(): void;
  /** Commit now, replacing any scheduled commit (final results and errors). */
  flush(): void;
  /** Drop any scheduled commit and ignore every later call (abort/unmount). */
  cancel(): void;
}

/**
 * A hidden tab runs no animation frames, so a timer also commits; whichever fires first
 * wins. Browsers clamp background timers to about a second, which still lets a hidden
 * load finish publishing.
 */
export const PUBLISH_FALLBACK_MS = 100;

const browserClock = (): FrameClock => ({
  requestFrame:
    typeof requestAnimationFrame === "function"
      ? (run) => requestAnimationFrame(run)
      : undefined,
  cancelFrame:
    typeof cancelAnimationFrame === "function"
      ? (id) => cancelAnimationFrame(id)
      : undefined,
  setTimer: (run, ms) => setTimeout(run, ms),
  clearTimer: (id) => clearTimeout(id),
});

/**
 * Coalesces one load's view publications to at most one `commit` per animation frame.
 * `commit` reads the latest state when it runs, so dropped intermediate calls lose nothing.
 */
export function publishScheduler(
  commit: () => void,
  clock: FrameClock = browserClock(),
): PublishScheduler {
  let frame: number | undefined;
  let timer: Timer | undefined;
  let pending = false;
  let stopped = false;
  const clear = () => {
    if (frame !== undefined) clock.cancelFrame?.(frame);
    if (timer !== undefined) clock.clearTimer(timer);
    frame = timer = undefined;
    pending = false;
  };
  const run = () => {
    if (!pending) return;
    clear();
    commit();
  };
  return {
    schedule() {
      if (stopped || pending) return;
      pending = true;
      frame = clock.requestFrame?.(run);
      timer = clock.setTimer(run, PUBLISH_FALLBACK_MS);
    },
    flush() {
      if (stopped) return;
      clear();
      commit();
    },
    cancel() {
      stopped = true;
      clear();
    },
  };
}
