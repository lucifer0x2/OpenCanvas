import { checkedResourceBytes } from "../../../renderer/resource-bytes.ts";
import type { FontManifest } from "../../../renderer/types.ts";

/** Bounded cache of validated bytes; rejected loads are evicted so Retry really retries. */
export class FontByteCache {
  private entries = new Map<string, Promise<ArrayBuffer>>();
  constructor(
    private fetchBytes = checkedResourceBytes,
    private capacity = 32,
  ) {}

  get = (font: FontManifest, timeoutMs = 10_000): Promise<ArrayBuffer> => {
    const key = JSON.stringify([font.id, font.hash, font.url]);
    const cached = this.entries.get(key);
    if (cached) {
      this.entries.delete(key);
      this.entries.set(key, cached);
      return cached;
    }
    const pending = this.fetchBytes(font, timeoutMs).catch((error) => {
      if (this.entries.get(key) === pending) this.entries.delete(key);
      throw error;
    });
    this.entries.set(key, pending);
    if (this.entries.size > this.capacity)
      this.entries.delete(this.entries.keys().next().value!);
    return pending;
  };
}

export const sceneFontBytes = new FontByteCache();

/** Stop this consumer without aborting an immutable byte fetch shared with another scene. */
export function untilAborted<T>(
  promise: Promise<T>,
  signal: AbortSignal,
): Promise<T> {
  signal.throwIfAborted();
  return new Promise((resolve, reject) => {
    const abort = () => reject(signal.reason);
    signal.addEventListener("abort", abort, { once: true });
    promise
      .then(resolve, reject)
      .finally(() => signal.removeEventListener("abort", abort));
  });
}
