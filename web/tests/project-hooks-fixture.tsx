import { StrictMode, useCallback, useState } from "react";
import { createRoot } from "react-dom/client";
import { useSWRConfig } from "swr";
import { useProject } from "../src/hooks/useProject";
import { useProjectList } from "../src/hooks/useProjectList";
import { useProjectLive } from "../src/hooks/useProjectLive";
import { useAcceptedSnapshot, useSnapshot } from "../src/hooks/useSnapshot";
import { SessionDataProvider } from "../src/data/SessionDataProvider";
import type { ProjectView } from "../src/app/projectView";
import type { ProjectRef, Selection } from "../src/types";

// Deliberately deliver queued callbacks and HTTP responses after cancellation: browsers
// can queue work before cleanup, and the hooks must still reject obsolete ownership.
const requests: { path: string; signal?: AbortSignal | null; validator: string | null; done: boolean; reply: (value: Response) => void }[] = [];
globalThis.fetch = (input, init) => new Promise<Response>((reply, reject) => {
  requests.push({ path: String(input), signal: init?.signal, validator: new Headers(init?.headers).get("If-None-Match"), done: false, reply });
  init?.signal?.addEventListener("abort", () => reject(new DOMException("Aborted", "AbortError")));
});
class FakeSource {
  static CLOSED = 2;
  static all: FakeSource[] = [];
  readyState = 0;
  closed = false;
  onopen?: () => void;
  onerror?: () => void;
  listeners = new Map<string, ((event: MessageEvent) => void)[]>();
  constructor(public url: string) { FakeSource.all.push(this); }
  close() { this.closed = true; this.readyState = 2; }
  addEventListener(name: string, callback: (event: MessageEvent) => void) {
    this.listeners.set(name, [...(this.listeners.get(name) ?? []), callback]);
  }
  emit(name: string, data: unknown) {
    if (name === "open") this.onopen?.();
    else if (name === "error") this.onerror?.();
    else for (const listener of this.listeners.get(name) ?? [])
      listener(new MessageEvent(name, { data: JSON.stringify(data) }));
  }
}
Object.defineProperty(globalThis, "EventSource", { value: FakeSource, configurable: true });

let current: ProjectView & { projects: ProjectRef[] };
let previous: typeof current | undefined;
let snapshotChanges = 0;
let projectChanges = 0;
let cacheValues: () => unknown[];
function Probe() {
  const { cache } = useSWRConfig();
  cacheValues = () => [...cache.keys()].flatMap((key) => {
    const snapshot = cache.get(key)?.data?.snapshot;
    return snapshot ? [{ key, projectId: snapshot.projectId, revision: snapshot.revision }] : [];
  });
  const list = useProjectList();
  const navigation = useProject(list.projects, list.loaded);
  const snapshot = useSnapshot(navigation.selection.projectId,
    list.projects.find((project) => project.projectId === navigation.selection.projectId)?.revision ?? null);
  useAcceptedSnapshot(snapshot.snapshot, snapshot.validation, navigation.accepted);
  const live = useProjectLive(list, navigation, snapshot);
  const selectScope = useCallback((patch: Partial<Selection>) => navigation.selectScope(snapshot.snapshot, patch),
    [navigation.selectScope, snapshot.snapshot]);
  current = { ...live, projects: list.projects, selection: navigation.selection,
    snapshot: snapshot.snapshot, selectScope, notice: navigation.notice, error: list.error || snapshot.error };
  if (current.snapshot !== previous?.snapshot) snapshotChanges++;
  if (current.projects !== previous?.projects) projectChanges++;
  previous = current;
  return <output>{current.selection.projectId}:{current.snapshot?.revision}</output>;
}
localStorage.clear();
localStorage.setItem("opencanvas.selection", JSON.stringify({
  projectId: "p", pageId: "page", frameId: "frame", nodeId: "node",
}));
const root = createRoot(document.getElementById("root")!);
let authorize: (value: boolean) => void;
function Session() {
  const [open, setOpen] = useState(true);
  authorize = setOpen;
  return open ? <SessionDataProvider><Probe /></SessionDataProvider> : <p>Locked</p>;
}
root.render(<StrictMode><Session /></StrictMode>);
Object.assign(window, { projectProbe: {
  requests: () => requests.map(({ path, signal, validator, done }, id) => ({ id, path, validator, done, aborted: signal?.aborted })),
  sources: () => FakeSource.all.map(({ url, closed }) => ({ url, closed })),
  reply(id: number, body: unknown, etag?: string, status = 200) {
    const request = requests[id];
    request.done = true;
    request.reply(new Response(status === 304 ? null : JSON.stringify(body), {
      status, headers: etag ? { ETag: etag } : {},
    }));
  },
  emit: (id: number, name: string, data?: unknown) => FakeSource.all[id].emit(name, data),
  value: () => ({ ...current, snapshotChanges, projectChanges }),
  select: (id: string) => current.selectProject(id),
  scope: (patch: Parameters<typeof current.selectScope>[0]) => current.selectScope(patch),
  follow: (next: boolean) => current.setFollow(next),
  retry: () => current.retry(),
  cache: () => cacheValues(),
  authorize: (open: boolean) => authorize(open),
  unmount: () => root.unmount(),
} });
