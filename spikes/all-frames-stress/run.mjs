import assert from "node:assert/strict";
import { mkdir, readFile } from "node:fs/promises";
import { arch, cpus, platform, release, totalmem } from "node:os";
import { connectMcp } from "../e2e/protocol.mjs";
import { PROFILES, seed } from "./fixture.mjs";
import { startBackend, OUT, save, rss } from "./runtime.mjs";
import { nativePage, snapshot } from "./native.mjs";
import { initializeWasm, wasmPage } from "./wasm.mjs";
import { preview } from "./preview.mjs";
import { cloneDataRoot } from "./clone-data.mjs";

await mkdir(OUT, { recursive: true });
let previous;
if (process.argv.includes("--resume")) {
  previous = JSON.parse(await readFile(`${OUT}/measurements.json`, "utf8"));
  await save(`previous-attempt-${Date.now()}.json`, previous);
}
let backend = await startBackend(previous?.backend.dataRoot);
let mcp, viewer, previewBackend;
const report = {
  capturedAt: new Date().toISOString(),
  runtime: {
    node: process.version,
    platform: platform(),
    arch: arch(),
    release: release(),
    cpu: cpus()[0].model,
    cores: cpus().length,
    memoryBytes: totalmem(),
    nativeProfile: "Debug (unoptimized + debuginfo)",
    wasmProfile: "prebuilt release artifact",
    nativePollMs: 500,
  },
  backend,
  rows: [],
};
const stop = async () => {
  await viewer?.stop();
  await mcp?.close();
  await backend.stop();
  if (previewBackend && previewBackend !== backend) await previewBackend.stop();
};
process.on("SIGINT", async () => {
  await stop();
  process.exit(0);
});
process.on("SIGTERM", async () => {
  await stop();
  process.exit(0);
});
try {
  mcp = await connectMcp(backend.base);
  const wasm = await initializeWasm();
  report.wasmInitializationMs = wasm.initMs;
  const existing = (await mcp.call("list_projects", {})).projects;
  for (const profile of Object.keys(PROFILES)) {
    console.log(`SEED ${profile}`);
    const saved = previous?.rows.find((row) => row.profile === profile);
    const found = existing.find(
      (project) => project.slug === `stress-${profile}`,
    );
    const fixture = found
      ? {
          profile,
          projectId: found.projectId,
          revision: found.revision,
          heights: PROFILES[profile],
          seedMs: saved?.seedMs ?? null,
          resumed: true,
        }
      : await seed(mcp, profile);
    const capture = await snapshot(backend.base, fixture);
    const browserEngine =
      saved?.wasm ??
      (await wasmPage(wasm.module, backend.base, fixture, capture.value));
    const row = {
      ...fixture,
      wasmReused: Boolean(saved?.wasm),
      snapshot: capture.metrics,
      native: saved?.native ?? null,
      wasm: browserEngine,
      serverRssBytes: rss(backend.pid),
    };
    report.rows.push(row);
    await save("measurements.json", report);
    console.log(
      `PAGE ${profile} seed=${fixture.seedMs}ms wasm=${browserEngine.elapsedMs.toFixed(1)}ms`,
    );
  }
  const existingBoundary = existing.find(
    (project) => project.slug === "stress-boundary",
  );
  const boundary = existingBoundary
    ? {
        profile: "boundary",
        projectId: existingBoundary.projectId,
        revision: existingBoundary.revision,
        heights: [6000],
        resumed: true,
      }
    : await seed(mcp, "boundary", [6000]);
  const response = await fetch(
    `${backend.base}/api/v1/projects/${boundary.projectId}/scene?frameId=frame-0&revision=${boundary.revision}&width=800&height=6000`,
  );
  const value = await response.json();
  report.boundary = {
    fixture: boundary,
    httpStatus: response.status,
    response: value,
  };
  assert.match(JSON.stringify(value), /2048/);
  await save("measurements.json", report);
  // Browser timings must not overlap asynchronous commit prerenders from seeding.
  await mcp.close();
  await backend.stop();
  backend = await startBackend(backend.dataRoot);
  previewBackend = backend;
  mcp = await connectMcp(backend.base);
  viewer = await preview(backend.base, report.rows);
  await save("session.json", {
    previewUrl: viewer.url,
    backendUrl: backend.base,
    backendPid: backend.pid,
    dataRoot: backend.dataRoot,
    harnessPid: process.pid,
  });
  console.log(`PREVIEW_URL=${viewer.url}`);
  console.log(
    "Browser autoruns four profiles and posts browser.json. Native remaining runs begin only after browser completes to avoid CPU contention.",
  );
  const browserResults = await viewer.browserComplete;
  report.browserProfiles = browserResults.map(({ profile, error }) => ({
    profile,
    error,
  }));
  const external = await readFile(`${OUT}/native.json`, "utf8")
    .then(JSON.parse)
    .catch((error) => {
      if (error.code === "ENOENT") return [];
      throw error;
    });
  for (const row of report.rows) {
    const found = external.find(
      (item) =>
        item.profile === row.profile && item.projectId === row.projectId,
    );
    if (!row.native && found) row.native = found;
  }
  for (const row of report.rows.filter((entry) => !entry.native)) {
    // Fresh process clears seed prerenders and previous page jobs; SQLite stays intact.
    await mcp.close();
    if (backend !== previewBackend) await backend.stop();
    backend = await startBackend(await cloneDataRoot(previewBackend.dataRoot));
    mcp = await connectMcp(backend.base);
    console.log(`NATIVE ${row.profile}`);
    row.native = await nativePage(backend.base, row, backend.pid);
    row.native.seedPrerendersExcluded = true;
    await save("measurements.json", report);
    console.log(
      `NATIVE_RESULT ${row.profile} elapsed=${row.native.elapsedMs.toFixed(1)}ms errors=${row.native.errors.length}`,
    );
  }
  console.log(
    "NATIVE_COMPLETE. Ctrl-C stops temporary processes; SQLite retained for review.",
  );
  await new Promise(() => {});
} catch (error) {
  report.failure = error.stack;
  await save("measurements.json", report);
  await stop();
  throw error;
}
