// Bounded wait on an in-page promise such as an animation's `finished`.
import assert from "node:assert/strict";

export const SETTLE_MS = 10_000;

const SNAPSHOT = `({ timeline: document.timeline.currentTime, visibility: document.visibilityState,
  animations: document.getAnimations().map((a) => [a.id, a.playState, a.pending, a.currentTime, a.playbackRate]) })`;

/**
 * Awaits `promise` (a page expression) and fails with every animation's state after SETTLE_MS
 * instead of hanging; a node-side timer also covers a page that stops answering.
 *
 * Frames keep coming while it waits, as in a visible tab. Chromium can start a reversed
 * composited animation (hover leave on `scale`) outside a frame and then schedule none: the
 * animation reads `finished` with its time running below 0, but its `finished` promise only
 * resolves on the next frame, which an idle headless page never draws.
 */
export async function settle(page, label, promise) {
  let timer;
  const stalled = new Promise((resolve) => {
    timer = setTimeout(() => resolve("page did not answer"), SETTLE_MS + 5_000);
  });
  const state = await Promise.race([
    page.evaluate(`new Promise((resolve) => {
      let open = true;
      const done = (value) => { if (open) { open = false; resolve(value); } };
      const pump = () => { if (open) requestAnimationFrame(pump); };
      pump();
      (${promise}).then(() => done(null), (error) => done({ rejected: String(error) }));
      setTimeout(() => done(${SNAPSHOT}), ${SETTLE_MS});
    })`),
    stalled,
  ]).finally(() => clearTimeout(timer));
  assert.equal(
    state,
    null,
    `${label} not settled after ${SETTLE_MS}ms: ${JSON.stringify(state)}`,
  );
}
