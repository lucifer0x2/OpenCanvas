import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { createServer } from "node:http";
import { readFile, writeFile } from "node:fs/promises";
import { platform, arch, release, cpus } from "node:os";
import {
  playwrightRuntime,
  PIN,
  ROOT,
} from "../native-render-parity/environment.mjs";
import { compare, distribution, PROVENANCE_EXCLUSIONS } from "./compare.mjs";

const out = `${ROOT}artifacts/viewer-wasm-scene/`;
const capture = JSON.parse(await readFile(`${out}native.json`, "utf8"));
assert.equal(capture.cases.length, 20, "never skip any requested matrix case");
assert.equal(new Set(capture.cases.map((c) => c.frameId)).size, 5);
const files = new Map([
  [
    "/worker.mjs",
    [`${ROOT}spikes/viewer-wasm-scene/worker.mjs`, "text/javascript"],
  ],
  ...["opencanvas_scene_wasm.js", "opencanvas_scene_wasm_bg.wasm"].map(
    (name) => [
      `/scene-wasm/${name}`,
      [
        `${ROOT}web/public/scene-wasm/${name}`,
        name.endsWith("wasm") ? "application/wasm" : "text/javascript",
      ],
    ],
  ),
  ...capture.cases.flatMap((c) =>
    c.fonts.map((f) => [
      `/artifacts/${f.file}`,
      [`${out}${f.file}`, "application/octet-stream"],
    ]),
  ),
]);
const server = createServer(async (req, res) => {
  if (req.url === "/")
    return res
      .writeHead(200, { "content-type": "text/html" })
      .end("<!doctype html><title>Scene parity</title>");
  const target = files.get(req.url);
  if (!target) return res.writeHead(404).end();
  try {
    res
      .writeHead(200, {
        "content-type": target[1],
        "cache-control": "no-store",
      })
      .end(await readFile(target[0]));
  } catch {
    res.writeHead(500).end();
  }
});
await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
let browser;
try {
  const { chromium } = await playwrightRuntime();
  browser = await chromium.launch({ headless: true });
  assert.equal(browser.version(), PIN.chromium);
  const page = await browser.newPage({
    viewport: { width: 1440, height: 900 },
  });
  await page.goto(`http://127.0.0.1:${server.address().port}/`);
  const rows = [];
  for (const fixture of capture.cases) {
    // JS parses original native server HTTP bytes; no Rust decimal parse/reencode.
    fixture.native = JSON.parse(
      await readFile(`${out}${fixture.nativeFile}`, "utf8"),
    );
    fixture.snapshot = JSON.parse(
      await readFile(`${out}${fixture.snapshotFile}`, "utf8"),
    );
    const provenance = fixture.native.scene.provenance;
    assert.equal(provenance.projectId, fixture.snapshot.projectId);
    assert.equal(provenance.revision, fixture.snapshot.revision);
    assert.equal(provenance.frameId, fixture.frameId);
    assert.deepEqual(provenance.viewport, {
      width: fixture.width,
      height: fixture.height,
    });
    assert.equal(provenance.state, "default");
    assert.equal(provenance.sourceSceneKey, fixture.native.sceneKey);
    const result = await page.evaluate(
      (fixture) =>
        new Promise((resolve, reject) => {
          const worker = new Worker("/worker.mjs", { type: "module" });
          const timeout = setTimeout(() => {
            worker.terminate();
            reject(new Error("Worker timeout"));
          }, 60000);
          worker.onerror = (e) => {
            clearTimeout(timeout);
            worker.terminate();
            reject(new Error(e.message));
          };
          worker.onmessage = ({ data }) => {
            clearTimeout(timeout);
            worker.terminate();
            resolve(data);
          };
          worker.postMessage(fixture);
        }),
      fixture,
    );
    assert.ok(!result.error, JSON.stringify(result.error));
    const comparison = compare(fixture.native.scene, result.scene);
    const row = {
      label: fixture.label,
      width: fixture.width,
      height: fixture.height,
      ...comparison,
      timings: { ...result.timings, warm: distribution(result.timings.warmMs) },
      worker: result.worker,
      wasm: result.scene,
      nativePaintError: fixture.native.error ?? null,
    };
    rows.push(row);
    console.log(
      JSON.stringify({
        label: row.label,
        nodes: row.nodes,
        texts: row.texts,
        lines: row.lines,
        mismatches: row.mismatches.length,
        coldMs: row.timings.coldMs,
        warm: row.timings.warm,
      }),
    );
  }
  const hashes = {};
  for (const file of [
    "web/public/scene-wasm/opencanvas_scene_wasm.js",
    "web/public/scene-wasm/opencanvas_scene_wasm_bg.wasm",
    "Cargo.lock",
    "crates/scene-wasm/src/engine.rs",
    "crates/server/src/jobs/native.rs",
    "crates/scene-input/src/input.rs",
    "crates/layout/src/scene_builder.rs",
    "crates/text/src/measurer.rs",
    "crates/server/tests/native_layout_support/fixtures.rs",
    "crates/server/tests/native_layout_support/fonts.rs",
    "samples/welcome/design.mjs",
    "samples/welcome-stacks/design.mjs",
  ]) {
    hashes[file] = createHash("sha256")
      .update(await readFile(`${ROOT}${file}`))
      .digest("hex");
  }
  const report = {
    capturedAt: new Date().toISOString(),
    runtime: {
      node: process.version,
      platform: platform(),
      arch: arch(),
      release: release(),
      cpu: cpus()[0]?.model,
      browser: browser.version(),
      pins: PIN,
      browserViewport: { width: 1440, height: 900 },
    },
    hashes,
    nativeExecutable: capture.executableSha256,
    exclusions: {
      provenance: PROVENANCE_EXCLUSIONS,
      outerJob:
        "jobId/status/readiness/metrics/image/PNG/error/transport not part of computed Scene",
    },
    rows,
  };
  await writeFile(`${out}comparison.json`, JSON.stringify(report, null, 2));
  const mismatchCount = rows.reduce((n, row) => n + row.mismatches.length, 0);
  console.log(`COMPARISONS=${rows.length} MISMATCHES=${mismatchCount}`);
  assert.equal(mismatchCount, 0, `Full mismatch ledger: ${out}comparison.json`);
} finally {
  await browser?.close();
  await new Promise((resolve) => {
    server.close(resolve);
    server.closeAllConnections();
  });
}
