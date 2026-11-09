import assert from "node:assert/strict";
import test from "node:test";
import { ApiError, VIEWER_SESSION_CHECK } from "../src/api.ts";
import { readSnapshot } from "../src/snapshotHttp.ts";
import type { AcceptedSnapshot } from "../src/snapshotHttp.ts";
import {
  entry,
  json,
  mockWindow,
  notModified,
  signal,
  snapshot,
} from "./snapshot-http-support.ts";

test("200 then 304 keeps the accepted snapshot identity and revalidates over HTTP", async (t) => {
  const requests: RequestInit[] = [];
  const responses = [json(snapshot(), '"one"'), notModified()];
  t.mock.method(
    globalThis,
    "fetch",
    async (path: string, init: RequestInit) => {
      assert.equal(path, "/api/v1/projects/p/snapshot");
      requests.push(init);
      return responses.shift()!;
    },
  );
  const first = await readSnapshot("p", null, signal());
  const second = await readSnapshot("p", first, signal());
  assert.ok(first);
  assert.equal(second, first);
  assert.equal(second.snapshot, first.snapshot);
  assert.equal(new Headers(requests[0].headers).has("If-None-Match"), false);
  assert.equal(new Headers(requests[1].headers).get("If-None-Match"), '"one"');
  assert.equal(requests[1].cache, "no-store");
  assert.equal(requests.length, 2);
});

test("200 with the same revision accepts changed privacy and metadata under a new ETag", async (t) => {
  const previous = entry();
  t.mock.method(globalThis, "fetch", async () =>
    json(
      {
        ...snapshot(),
        privacy_status: "private",
        name: "Renamed",
        retention: { oldestCommitRevision: 2, idempotencyPolicy: "changed" },
      },
      '"two"',
    ),
  );
  const next = await readSnapshot("p", previous, signal());
  assert.ok(next);
  assert.notEqual(next.snapshot, previous.snapshot);
  assert.equal(next.snapshot.revision, previous.snapshot.revision);
  assert.equal(next.snapshot.name, "Renamed");
  assert.equal(next.snapshot.retention.oldestCommitRevision, 2);
  assert.equal(Reflect.get(next.snapshot, "privacy_status"), "private");
  assert.equal(next.etag, '"two"');
  assert.equal(previous.etag, '"one"');
});

test("a missing or weak ETag preserves unconditional 200 behavior", async (t) => {
  for (const etag of [undefined, 'W/"weak"']) {
    const requests: RequestInit[] = [];
    t.mock.method(
      globalThis,
      "fetch",
      async (_path: string, init: RequestInit) => {
        requests.push(init);
        return json(snapshot(), etag);
      },
    );
    const first = await readSnapshot("p", null, signal());
    const second = await readSnapshot("p", first, signal());
    assert.ok(first && second);
    assert.equal(first.etag, null);
    assert.notEqual(first.snapshot, second.snapshot);
    assert.equal(new Headers(requests[1].headers).has("If-None-Match"), false);
    t.mock.restoreAll();
  }
});

test("foreign-project and backwards responses cannot replace snapshot or validator", async (t) => {
  const previous = entry();
  for (const body of [
    snapshot({ projectId: "other" }),
    snapshot({ revision: 2 }),
  ]) {
    t.mock.method(globalThis, "fetch", async () => json(body, '"ignored"'));
    assert.equal(await readSnapshot("p", previous, signal()), null);
    assert.equal(previous.etag, '"one"');
    assert.equal(previous.snapshot.revision, 3);
    t.mock.restoreAll();
  }
});

test("truncated responses retain the existing incomplete-snapshot error", async (t) => {
  const previous = entry();
  t.mock.method(globalThis, "fetch", async () =>
    json(snapshot({ truncated: true }), '"bad"'),
  );
  await assert.rejects(
    readSnapshot("p", previous, signal()),
    /This snapshot is incomplete/,
  );
  assert.equal(previous.etag, '"one"');
});

test("project changes and an empty accepted state never reuse another validator", async (t) => {
  const requests: RequestInit[] = [];
  t.mock.method(
    globalThis,
    "fetch",
    async (_path: string, init: RequestInit) => {
      requests.push(init);
      return json(snapshot({ projectId: "q" }), '"q-tag"');
    },
  );
  const next = await readSnapshot("q", entry(), signal());
  await readSnapshot("q", null, signal());
  assert.equal(next?.snapshot.projectId, "q");
  for (const request of requests)
    assert.equal(new Headers(request.headers).has("If-None-Match"), false);
});

test("unexpected or mismatched 304 fails once without parsing or retry loops", async (t) => {
  let count = 0;
  t.mock.method(globalThis, "fetch", async () => {
    count++;
    return notModified();
  });
  const invalid: (AcceptedSnapshot | null)[] = [
    null,
    { snapshot: snapshot({ projectId: "q" }), etag: '"one"' },
    { snapshot: snapshot(), etag: null },
    { snapshot: snapshot(), etag: 'W/"one"' },
    { snapshot: snapshot(), etag: '"different"' },
  ];
  for (const previous of invalid)
    await assert.rejects(readSnapshot("p", previous, signal()), {
      code: "INVALID_SNAPSHOT_RESPONSE",
      status: 304,
    });
  assert.equal(count, invalid.length);
});

test("401 still requests a viewer session check, and 404 retains its clearing status", async (t) => {
  const events: string[] = [];
  mockWindow(t, (event) => {
    events.push(event.type);
    return true;
  });
  for (const status of [401, 404]) {
    t.mock.method(globalThis, "fetch", async () =>
      json(
        {
          error: { code: "NOT_AVAILABLE", message: "Unavailable" },
        },
        undefined,
        status,
      ),
    );
    await assert.rejects(readSnapshot("p", entry(), signal()), (cause) => {
      assert.ok(cause instanceof ApiError);
      assert.equal(cause.status, status);
      assert.equal(cause.code, "NOT_AVAILABLE");
      assert.equal(cause.message, "Unavailable");
      return true;
    });
    t.mock.restoreAll();
  }
  assert.deepEqual(events, [VIEWER_SESSION_CHECK]);
});

test("abort prevents late responses and validators from being accepted", async (t) => {
  const controller = new AbortController();
  const previous = entry();
  t.mock.method(globalThis, "fetch", async () => {
    controller.abort();
    return json(snapshot({ revision: 4 }), '"late"');
  });
  await assert.rejects(readSnapshot("p", previous, controller.signal), {
    name: "AbortError",
  });
  assert.equal(previous.etag, '"one"');
  assert.equal(previous.snapshot.revision, 3);
});

test("aborted 401 does not request a new session check", async (t) => {
  const controller = new AbortController();
  mockWindow(t, () => {
    throw new Error("Aborted session event");
  });
  t.mock.method(globalThis, "fetch", async () => {
    controller.abort();
    return json({}, undefined, 401);
  });
  await assert.rejects(readSnapshot("p", entry(), controller.signal), {
    name: "AbortError",
  });
});
