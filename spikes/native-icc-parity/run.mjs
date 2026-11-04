import { readFile, writeFile, mkdir, readdir } from "node:fs/promises";
import { dirname, join, relative, resolve } from "node:path";
import { fileURLToPath } from "node:url";
import os from "node:os";
import { withOracle } from "../native-paint-parity/oracle.mjs";
import { hash } from "../native-paint-parity/scene.mjs";
import { gate } from "../native-paint-parity/compare.mjs";
import { identities, changed, build, browserIdentity } from "./runtime.mjs";
import { one, json } from "./capture.mjs";
import { effect } from "./reference.mjs";
const here = dirname(fileURLToPath(import.meta.url)), root = resolve(here, "../..");
const out = join(here, "out", new Date().toISOString().replaceAll(":", "-"));
await mkdir(out, { recursive: true });
const report = { schema: "native-icc-parity/1", createdAt: new Date().toISOString(), gate,
  pixelPolicy: "All pixels, straight RGBA8, any channel >8; fraction strictly <0.005. Ignore RGB only if both alpha=0. No text, masks, resizing or corners.",
  os: { platform: os.platform(), release: os.release(), arch: os.arch(), version: os.version() }, node: process.version,
  sourceHashesBefore: await identities(root, here), cases: [], effects: [], probes: [],
  limitations: ["Synthetic CMYK constant-PCS checks ICC application/opaque K, not a real printer profile or Adobe/YCCK inversion convention.",
    "No WebP ICC, non-RGB/gray/CMYK spaces, orientation+profile, scaling, or printer intent coverage.",
    "Browser has no exposed ICC parser. Validity evidence is successful original-byte decode plus analytic reference and tagged/untagged effect, not decode success alone.",
    "RGBA export quantizes through premultiplied Canvas/native surfaces; exact alpha and independent RGB reference tolerance 4 are separate from the unchanged parity gate."],
};
try {
  const native = await build(here, out); report.native = native;
  const fixtures = JSON.parse(await readFile(join(out, "fixtures.json")));
  report.profiles = [];
  for (const name of await readdir(join(out, "profiles"))) {
    const bytes = await readFile(join(out, "profiles", name));
    report.profiles.push({ name, sha256: hash(bytes), bytes: bytes.length, origin: "MIT repository-authored moxcms profile math", parse: "moxcms valid" });
  }
  await withOracle(root, out, async oracle => {
    const { page, browserVersion, playwrightVersion, rendererBundleHash, browserExecutable } = oracle;
    // Playwright may launch headless_shell rather than chromium.executablePath().
    report.oracle = { browserVersion, playwrightVersion, rendererBundleHash, configuredExecutable: browserExecutable,
      ...await browserIdentity(page.context().browser()), colorProfile: "sRGB", deviceScaleFactor: 1 };
    await oracle.refreshResources();
    const captures = new Map();
    for (const f of fixtures) {
      const result = await one(page, native, out, f); report.cases.push(result.row); captures.set(f.id, result);
    }
    for (const f of fixtures.filter(f => f.tagged)) {
      const control = captures.get(f.id.replace(/-icc$/, "-untagged")), tagged = captures.get(f.id);
      for (const engine of ["chrome", "native"]) {
        const a = tagged.rasters[engine], b = control.rasters[engine];
        report.effects.push({ pair: f.pair, engine, ...(a && b ? effect(f.profile, a, b) : { pass: false, reason: "Decode error" }) });
      }
    }
    const probe = await one(page, native, out, { id: "malformed-icc", file: "malformed-icc.png", mediaType: "image/png", width: 32, height: 16, probe: true });
    report.probes.push(probe.row);
  });
  report.lockHashAfter = hash(await readFile(join(here, "Cargo.lock")));
  report.dependencyLockChangedDuringCapture = report.lockHashAfter !== report.native.lockHash;
} catch (error) {
  report.error = { message: error.message, stack: error.stack }; process.exitCode = 1;
} finally {
  report.sourceHashesAfter = await identities(root, here);
  report.changedSources = changed(report.sourceHashesBefore, report.sourceHashesAfter);
  const failed = report.cases.filter(c => c.verdict !== "PASS");
  report.summary = { cases: report.cases.length, pass: report.cases.length - failed.length,
    fail: failed.length, effectFailures: report.effects.filter(e => !e.pass).length,
    drawStatsMismatches: report.cases.filter(c => c.drawStatsEqual === false).length,
    probes: report.probes.length, probeFailures: report.probes.filter(p => !p.nativeRejectsMalformedICC).length };
  report.status = report.error ? "HARNESS_ERROR" : report.changedSources.length || report.dependencyLockChangedDuringCapture ? "INCONCLUSIVE_SOURCE_DRIFT" :
    failed.length || report.summary.effectFailures || report.summary.probeFailures || report.summary.drawStatsMismatches ? "FAIL" : "PASS";
  if (report.status !== "PASS" && !process.exitCode) process.exitCode = 2;
  // Binary path is local evidence only; no project or production data is accessed.
  await json(join(out, "report.json"), report);
  await writeFile(join(here, "out/latest.json"), JSON.stringify({ path: relative(here, out) }) + "\n");
  console.log(JSON.stringify({ report: relative(root, join(out, "report.json")), status: report.status, summary: report.summary,
    changedSources: report.changedSources, error: report.error?.message }, null, 2));
}
