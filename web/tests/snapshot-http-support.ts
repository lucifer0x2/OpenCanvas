import type { TestContext } from "node:test";
import type { AcceptedSnapshot } from "../src/snapshotHttp.ts";
import type { Snapshot } from "../src/types.ts";

export const snapshot = (overrides: Partial<Snapshot> = {}): Snapshot => ({
  schemaVersion: 1,
  projectId: "p",
  slug: "project",
  name: "Project",
  revision: 3,
  nodes: [],
  tokens: [],
  styles: [],
  fonts: [],
  assets: [],
  truncated: false,
  retention: { oldestCommitRevision: 1, idempotencyPolicy: "bounded" },
  ...overrides,
});
export const signal = () => new AbortController().signal;
export const entry = (): AcceptedSnapshot => ({
  snapshot: snapshot(),
  etag: '"one"',
});
export const json = (body: unknown, etag?: string, status = 200) =>
  new Response(JSON.stringify(body), {
    status,
    headers: etag ? { ETag: etag } : {},
  });
export const notModified = (etag = '"one"') => {
  const response = new Response(null, { status: 304, headers: { ETag: etag } });
  response.json = async () => {
    throw new Error("A 304 body must never be parsed");
  };
  return response;
};
export const mockWindow = (
  t: TestContext,
  dispatchEvent: (event: Event) => boolean,
) => {
  const descriptor = Object.getOwnPropertyDescriptor(globalThis, "window");
  Object.defineProperty(globalThis, "window", {
    configurable: true,
    value: { dispatchEvent },
  });
  t.after(() => {
    if (descriptor) Object.defineProperty(globalThis, "window", descriptor);
    else Reflect.deleteProperty(globalThis, "window");
  });
};
