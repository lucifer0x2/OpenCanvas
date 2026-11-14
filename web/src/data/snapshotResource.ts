import { unstable_serialize, type Cache, type ScopedMutator } from "swr";
import { ApiError } from "../api";
import { snapshotNodeIndex } from "../nodeIndex";
import { readSnapshot, type AcceptedSnapshot } from "../snapshotHttp";

import {
  snapshotEpochKey,
  snapshotHeadKey,
  snapshotKey,
  snapshotReadKey,
  type SnapshotReadKey,
  type SnapshotPointer,
  type SnapshotHead,
} from "./snapshotKeys";

/** Only promises/key bookkeeping live here; all payloads and validators belong to SWR. */
class SnapshotResource {
  private pending = new Map<
    string,
    {
      promise: Promise<SnapshotPointer>;
      epoch: number;
      controller: AbortController;
    }
  >();
  private controllers = new Set<AbortController>();
  private reads = new Map<string, Set<string>>();
  private retained = new Map<string, string>();
  private validation = 0;
  private active = true;
  constructor(
    private cache: Cache,
    private mutate: ScopedMutator,
  ) {}

  private value<T>(key: readonly unknown[]): T | undefined {
    return this.cache.get(unstable_serialize(key))?.data as T | undefined;
  }
  current(project: string): AcceptedSnapshot | null {
    const head = this.value<SnapshotHead>(snapshotHeadKey(project));
    return head
      ? (this.value<AcceptedSnapshot>(snapshotKey(project, head.revision)) ??
          null)
      : null;
  }
  epoch(project: string) {
    return this.value<number>(snapshotEpochKey(project)) ?? 0;
  }

  attach() {
    this.active = true;
    return () => {
      this.active = false;
      // Effect replay reacquires the same provider before this microtask. A real
      // authorization unmount aborts its reads before the next network task runs.
      queueMicrotask(() => {
        if (!this.active)
          for (const controller of this.controllers) controller.abort();
      });
    };
  }

  async invalidate(project: string, visibilityChanged = false) {
    if (!project && !visibilityChanged) return;
    // Publish the epoch before clearing: an old in-flight response cannot repopulate access.
    const update = project
      ? this.mutate(snapshotEpochKey(project), this.epoch(project) + 1, {
          revalidate: false,
        })
      : undefined;
    this.pending.get(project)?.controller.abort();
    if (visibilityChanged) {
      for (const other of new Set([
        ...this.retained.keys(),
        ...this.pending.keys(),
      ])) {
        if (other === project) continue;
        void this.mutate(snapshotEpochKey(other), this.epoch(other) + 1, {
          revalidate: false,
        });
        this.pending.get(other)?.controller.abort();
        await this.clear(other);
      }
    }
    await update;
  }

  private async clear(project: string) {
    const head = this.value<SnapshotHead>(snapshotHeadKey(project));
    await this.mutate(snapshotHeadKey(project), undefined, {
      revalidate: false,
    });
    if (head)
      this.cache.delete(
        unstable_serialize(snapshotKey(project, head.revision)),
      );
    this.retained.delete(project);
    for (const key of this.reads.get(project) ?? []) this.cache.delete(key);
    this.reads.delete(project);
  }

  reconcileVisible(projects: readonly string[]) {
    const visible = new Set(projects);
    for (const project of new Set([
      ...this.retained.keys(),
      ...this.pending.keys(),
    ])) {
      if (visible.has(project)) continue;
      void this.mutate(snapshotEpochKey(project), this.epoch(project) + 1, {
        revalidate: false,
      });
      this.pending.get(project)?.controller.abort();
      void this.clear(project);
    }
  }

  fetch = async ([
    ,
    project,
    revision,
    epoch,
  ]: SnapshotReadKey): Promise<SnapshotPointer> => {
    const key = unstable_serialize(snapshotReadKey(project, revision, epoch));
    const reads = this.reads.get(project) ?? new Set<string>();
    reads.add(key);
    this.reads.set(project, reads);
    const head = this.value<SnapshotHead>(snapshotHeadKey(project));
    const current = this.current(project);
    if (
      current &&
      head?.epoch === epoch &&
      (revision === null || current.snapshot.revision >= revision)
    )
      return {
        revision: current.snapshot.revision,
        validation: this.validation,
      };
    // Different watermark keys read the same latest endpoint. Wait for that read,
    // then request again only when it did not satisfy the queued newer watermark.
    const pending = this.pending.get(project);
    if (pending) {
      try {
        await pending.promise;
      } catch (cause) {
        if (epoch <= pending.epoch) throw cause;
      }
      return this.fetch([
        "snapshot-latest",
        project,
        revision,
        epoch,
        "object",
        "visible",
      ]);
    }
    const controller = new AbortController();
    const request = this.load(project, epoch, controller);
    this.pending.set(project, { promise: request, epoch, controller });
    try {
      return await request;
    } finally {
      if (this.pending.get(project)?.promise === request)
        this.pending.delete(project);
    }
  };

  private async load(
    project: string,
    epoch: number,
    controller: AbortController,
  ): Promise<SnapshotPointer> {
    this.controllers.add(controller);
    try {
      const next = await readSnapshot(
        project,
        this.current(project),
        controller.signal,
      );
      if (!this.active || epoch !== this.epoch(project))
        return { revision: null, validation: ++this.validation };
      const latest = this.current(project);
      const accepted =
        next && (!latest || next.snapshot.revision >= latest.snapshot.revision)
          ? next
          : latest;
      if (accepted) {
        snapshotNodeIndex(accepted.snapshot);
        const key = snapshotKey(project, accepted.snapshot.revision);
        await this.mutate(key, accepted, { revalidate: false });
        await this.mutate(
          snapshotHeadKey(project),
          { revision: accepted.snapshot.revision, epoch },
          { revalidate: false },
        );
        this.retain(project, unstable_serialize(key));
      }
      return {
        revision: accepted?.snapshot.revision ?? null,
        validation: ++this.validation,
      };
    } catch (cause) {
      if (
        cause instanceof ApiError &&
        cause.status === 404 &&
        epoch === this.epoch(project)
      )
        await this.clear(project);
      throw cause;
    } finally {
      this.controllers.delete(controller);
    }
  }

  private retain(project: string, key: string) {
    const previous = this.retained.get(project);
    if (previous && previous !== key) this.cache.delete(previous);
    this.retained.delete(project);
    this.retained.set(project, key);
    // Eight latest project snapshots, no historical trees. Eviction also removes
    // request pointers, so an immutable request key cannot strand a missing payload.
    while (this.retained.size > 8) {
      const oldest = this.retained.keys().next().value!;
      void this.mutate(snapshotEpochKey(oldest), this.epoch(oldest) + 1, {
        revalidate: false,
      });
      this.cache.delete(this.retained.get(oldest)!);
      this.cache.delete(unstable_serialize(snapshotHeadKey(oldest)));
      for (const cached of this.reads.get(oldest) ?? [])
        this.cache.delete(cached);
      this.reads.delete(oldest);
      this.retained.delete(oldest);
    }
  }
}

const resources = new WeakMap<Cache, SnapshotResource>();
export function snapshotResource(cache: Cache, mutate: ScopedMutator) {
  let resource = resources.get(cache);
  if (!resource) {
    resource = new SnapshotResource(cache, mutate);
    resources.set(cache, resource);
  }
  return resource;
}
