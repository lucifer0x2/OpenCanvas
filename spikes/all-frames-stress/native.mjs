import assert from "node:assert/strict";
import { performance } from "node:perf_hooks";
import { json } from "../e2e/protocol.mjs";
import { pause, rss, save } from "./runtime.mjs";

export async function snapshot(base, fixture) {
  const url = `${base}/api/v1/projects/${fixture.projectId}/snapshot`;
  const start = performance.now(),
    response = await fetch(url);
  const bytes = await response.text(),
    elapsedMs = performance.now() - start;
  assert.equal(response.status, 200, bytes.slice(0, 500));
  const value = JSON.parse(bytes),
    etag = response.headers.get("etag");
  assert.equal(value.nodes.length, 10001);
  assert.equal(value.revision, fixture.revision);
  const conditionalStart = performance.now();
  const conditional = await fetch(url, { headers: { "if-none-match": etag } });
  await conditional.arrayBuffer();
  assert.equal(conditional.status, 304);
  const conditionalMs = performance.now() - conditionalStart;
  await save(`${fixture.profile}-snapshot.json`, value);
  return {
    value,
    metrics: {
      bytes: Buffer.byteLength(bytes),
      elapsedMs,
      etag,
      conditionalMs,
      conditionalStatus: conditional.status,
    },
  };
}

export async function nativePage(base, fixture, pid) {
  const start = performance.now(),
    jobs = [],
    batches = [],
    errors = [];
  let offset = 0,
    peakRssBytes = rss(pid);
  do {
    const begun = performance.now();
    const response = await fetch(
      `${base}/api/v1/projects/${fixture.projectId}/scenes?pageId=page&revision=${fixture.revision}&offset=${offset}&limit=4`,
      { signal: AbortSignal.timeout(240000) },
    );
    const batch = await response.json();
    const admissionMs = performance.now() - begun;
    if (!response.ok) {
      errors.push(batch);
      break;
    }
    assert.ok(batch.jobs.length <= 4);
    const completed = await Promise.all(
      batch.jobs.map(async (initial) => {
        let job = initial,
          polls = 0;
        const deadline = performance.now() + 240000;
        while (job.status === "pending" && performance.now() < deadline) {
          await pause(500);
          job = await json(base, `/api/v1/jobs/${job.jobId}`);
          polls++;
        }
        if (job.status === "pending")
          errors.push({
            frameId: job.frameId,
            error:
              "Still pending at 240s harness observation window; no subsequent batch admitted",
          });
        if (job.status !== "ready" || job.error)
          errors.push({ frameId: job.frameId, error: job.error });
        if (job.scene) {
          assert.equal(job.scene.provenance.revision, fixture.revision);
          assert.equal(job.scene.provenance.frameId, job.frameId);
          const index = Number(job.frameId.split("-").at(-1));
          assert.deepEqual(job.scene.provenance.viewport, {
            width: 800,
            height: fixture.heights[index],
          });
          assert.equal(job.scene.nodes.length, index === 0 ? 1112 : 1111);
        }
        return {
          frameId: job.frameId,
          status: job.status,
          nodes: job.scene?.nodes.length,
          revision: job.scene?.provenance.revision,
          elapsedFromBatchMs: performance.now() - begun,
          polls,
          metrics: job.metrics,
          image: job.image,
          error: job.error,
        };
      }),
    );
    jobs.push(...completed);
    peakRssBytes = Math.max(peakRssBytes, rss(pid));
    batches.push({
      offset,
      jobs: completed.length,
      admissionMs,
      elapsedMs: performance.now() - begun,
    });
    console.log(
      `NATIVE_BATCH ${fixture.profile} offset=${offset} jobs=${completed.length} elapsedMs=${(performance.now() - begun).toFixed(1)}`,
    );
    offset = batch.nextOffset;
    if (completed.some((job) => job.status === "pending")) break;
  } while (offset !== null);
  if (!errors.length) {
    assert.equal(jobs.length, 9);
    assert.equal(
      jobs.reduce((sum, job) => sum + job.nodes, 0),
      10000,
    );
  }
  return {
    elapsedMs: performance.now() - start,
    batches,
    jobs,
    errors,
    peakSampledRssBytes: peakRssBytes,
  };
}
