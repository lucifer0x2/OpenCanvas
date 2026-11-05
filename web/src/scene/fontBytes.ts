import { checkedResourceBytes } from "../../../renderer/resource-bytes.ts";
import type { FontManifest } from "../../../renderer/types.ts";

type Entry = { promise: Promise<ArrayBuffer>; byteLength: number };

/** Bounded cache of validated bytes; rejected loads are evicted so Retry really retries. */
export class FontByteCache {
  private entries = new Map<string, Entry>();
  private retainedBytes = 0;
  constructor(
    private fetchBytes = checkedResourceBytes,
    private capacity = 32,
    private byteCapacity = 64 * 1024 * 1024,
  ) {}

  get = (font: FontManifest, timeoutMs = 10_000): Promise<ArrayBuffer> => {
    const key = JSON.stringify([font.id, font.hash, font.url]);
    const cached = this.entries.get(key);
    if (cached) {
      this.entries.delete(key);
      this.entries.set(key, cached);
      return cached.promise;
    }
    const entry: Entry = {
      byteLength: 0,
      promise: this.fetchBytes(font, timeoutMs).then(
        (bytes) => {
          // An evicted load may finish after a replacement for this key started.
          if (this.entries.get(key) === entry) {
            if (bytes.byteLength > this.byteCapacity) this.remove(key);
            else {
              entry.byteLength = bytes.byteLength;
              this.retainedBytes += entry.byteLength;
              this.trim();
            }
          }
          return bytes;
        },
        (error) => {
          if (this.entries.get(key) === entry) this.remove(key);
          throw error;
        },
      ),
    };
    this.entries.set(key, entry);
    this.trim();
    return entry.promise;
  };

  private remove(key: string) {
    this.retainedBytes -= this.entries.get(key)?.byteLength ?? 0;
    this.entries.delete(key);
  }

  private trim() {
    while (
      this.entries.size &&
      (this.entries.size > this.capacity ||
        this.retainedBytes > this.byteCapacity)
    )
      this.remove(this.entries.keys().next().value!);
  }
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
