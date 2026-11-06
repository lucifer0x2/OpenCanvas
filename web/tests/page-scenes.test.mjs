import assert from "node:assert/strict";
import test from "node:test";
import { sceneModule } from "./scene-module.mjs";

const { loadPageScenes } = await sceneModule("pageScenes");
const requests = Array.from({ length: 9 }, (_, i) => ({
  projectId: "fixture",
  revision: 7,
  frameId: `frame-${i}`,
  width: 800,
  height: 600,
}));
const ready = (request) => ({
  ...request,
  jobId: request.frameId,
  status: "ready",
  scene: {
    provenance: {
      ...request,
      viewport: { width: request.width, height: request.height },
      theme: "default",
      state: "default",
      sceneVersion: "scene-3",
      rendererVersion: "web-4",
    },
  },
});
const json = (body) =>
  new Response(JSON.stringify(body), {
    headers: { "content-type": "application/json" },
  });

test("all four pending jobs finish before admission advances, including confirmed worker failures", async () => {
  const active = new Set(),
    offsets = [];
  const original = globalThis.fetch;
  globalThis.fetch = async (path) => {
    const url = new URL(path, "http://fixture");
    if (url.pathname.endsWith("/scenes")) {
      assert.equal(active.size, 0, "no pending jobs when next batch starts");
      const offset = Number(url.searchParams.get("offset"));
      offsets.push(offset);
      const batch = requests.slice(offset, offset + 4);
      batch.forEach((request) => active.add(request.frameId));
      return json({
        projectId: "fixture",
        revision: 7,
        pageId: "page",
        total: 9,
        nextOffset: offset + 4 < 9 ? offset + 4 : null,
        jobs: batch.map((request) => ({
          ...ready(request),
          status: "pending",
          scene: undefined,
        })),
      });
    }
    const frameId = url.pathname.split("/").at(-1);
    active.delete(frameId);
    return json(
      frameId === "frame-2"
        ? {
            ...ready(requests[2]),
            status: "failed",
            scene: undefined,
            error: { message: "Known worker failure" },
          }
        : ready(requests.find((request) => request.frameId === frameId)),
    );
  };
  try {
    const result = await loadPageScenes(
      requests,
      "page",
      {},
      new AbortController().signal,
      false,
    );
    assert.deepEqual(offsets, [0, 4, 8]);
    assert.equal(result.length, 9);
    assert.match(result[2].error, /Known worker failure/);
    assert.equal(result.filter((entry) => entry.job).length, 8);
  } finally {
    globalThis.fetch = original;
  }
});

test("unknown pending-job transport failure stops admission before offset4", async () => {
  for (const message of ["Connection interrupted"]) {
    const paths = [],
      original = globalThis.fetch;
    globalThis.fetch = async (path) => {
      paths.push(path);
      if (path.includes("/jobs/")) throw new Error(message);
      return json({
        projectId: "fixture",
        revision: 7,
        pageId: "page",
        total: 9,
        nextOffset: 4,
        jobs: requests.slice(0, 4).map((request) => ({
          ...ready(request),
          status: "pending",
          scene: undefined,
        })),
      });
    };
    try {
      await assert.rejects(
        loadPageScenes(
          requests,
          "page",
          {},
          new AbortController().signal,
          false,
        ),
        { message },
      );
      assert.equal(paths.filter((path) => path.includes("/scenes?")).length, 1);
      assert.ok(paths.every((path) => !path.includes("offset=4")));
    } finally {
      globalThis.fetch = original;
    }
  }
});

test("a real pending-job timeout stops admission even though the worker may still be running", async () => {
  const paths = [],
    originalFetch = globalThis.fetch,
    originalNow = Date.now;
  let clock = 0;
  Date.now = () => (clock += 50_000);
  globalThis.fetch = async (path) => {
    paths.push(path);
    return json({
      projectId: "fixture",
      revision: 7,
      pageId: "page",
      total: 9,
      nextOffset: 4,
      jobs: requests.slice(0, 4).map((request) => ({
        ...ready(request),
        status: "pending",
        scene: undefined,
      })),
    });
  };
  try {
    await assert.rejects(
      loadPageScenes(requests, "page", {}, new AbortController().signal, false),
      /Rendering timed out/,
    );
    assert.equal(
      paths.length,
      1,
      "no next batch or mistaken successful poll after unknown completion",
    );
  } finally {
    globalThis.fetch = originalFetch;
    Date.now = originalNow;
  }
});

test("missing frames, wrong identity or stale provenance fail visibly instead of dropping frames", async () => {
  const original = globalThis.fetch;
  for (const mutation of [
    (batch) => {
      batch.jobs.pop();
    },
    (batch) => {
      batch.projectId = "foreign";
    },
    (batch) => {
      batch.jobs[0].scene.provenance.revision = 6;
    },
  ]) {
    globalThis.fetch = async () => {
      const batch = {
        projectId: "fixture",
        revision: 7,
        pageId: "page",
        total: 9,
        nextOffset: 4,
        jobs: requests.slice(0, 4).map(ready),
      };
      mutation(batch);
      return json(batch);
    };
    try {
      await assert.rejects(
        loadPageScenes(
          requests,
          "page",
          {},
          new AbortController().signal,
          false,
        ),
      );
    } finally {
      globalThis.fetch = original;
    }
  }
});
