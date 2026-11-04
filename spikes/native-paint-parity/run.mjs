import { mkdir, writeFile, readFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { dirname, join, relative, resolve } from "node:path";
import os from "node:os";
import { fixtures } from "./fixtures.mjs";
import { probes } from "./probes.mjs";
import { withOracle, capture } from "./oracle.mjs";
import { prepareResources } from "./resources.mjs";
import { sourceHashes, buildNative, nativeCapture } from "./runtime.mjs";
import { decodePNG, encodePNG } from "./png.mjs";
import { compare, gate } from "./compare.mjs";
import { hash } from "./scene.mjs";

const here = dirname(fileURLToPath(import.meta.url)), root = resolve(here, "../..");
const wantNative = process.argv.includes("--native");
const out = join(here, "out", new Date().toISOString().replaceAll(":", "-"));
await mkdir(out, { recursive: true });
const writeJSON = (path, value) => writeFile(path, JSON.stringify(value, null, 2) + "\n");
const report = { schema: "native-paint-parity/1", createdAt: new Date().toISOString(), gate,
  pixelPolicy: "Every output pixel. Straight RGBA; RGB ignored only when both alpha=0. Raw RGBA and white-composited metrics also reported. No masks.",
  os: { platform: os.platform(), release: os.release(), arch: os.arch() }, node: process.version,
  sourceHashesBefore: await sourceHashes(root), cases: [], probes: [],
  limitations: ["No text mask: text gets full-image metrics and REVIEW_TEXT, never automatic acceptance.",
    "ICC originals not available in bounded licensed repository fixtures; ICC-specific gate remains uncovered.",
    "Image originals are locally generated; no external downloads or OS font lookup."],
};
try {
  const native = wantNative ? await buildNative(root, out) : null;
  if (native) report.native = { deps: native.deps, rustc: native.rustc, binaryHash: hash(await readFile(native.binary)) };
  await withOracle(root, out, async oracle => {
    const { page, browserVersion, playwrightVersion, rendererBundleHash, browserExecutable } = oracle;
    report.oracle = { browserVersion, playwrightVersion, rendererBundleHash, browserExecutable };
    const resources = await prepareResources(page, root, out); await oracle.refreshResources();
    report.resources = resources.sources;
    for (const fixture of fixtures(resources)) for (const scale of fixture.scales) {
      const key = `${fixture.name}@${scale}`, dir = join(out, key); await mkdir(dir);
      const request = join(dir, "request.json"); await writeJSON(request, { scene: fixture.scene, scale });
      const chrome = await capture(page, fixture.scene, scale);
      const row = { name: fixture.name, category: fixture.category, scale, sceneHash: hash(JSON.stringify(fixture.scene)),
        request: relative(out, request), chrome: { ...chrome, png: undefined } };
      report.cases.push(row);
      if (!chrome.ok) throw Error(`Chrome fixture ${key}: ${chrome.code}`);
      const chromeBytes = Buffer.from(chrome.png, "base64");
      await writeFile(join(dir, "chrome.png"), chromeBytes); row.chrome.pngHash = hash(chromeBytes);
      if (!native) { row.verdict = "NATIVE_PENDING"; continue; }
      row.native = await nativeCapture(native.binary, out, request, join(dir, "native.png"));
      if (!row.native.ok) { row.verdict = "NATIVE_ERROR"; continue; }
      const nativeBytes = await readFile(join(dir, "native.png")); row.native.pngHash = hash(nativeBytes);
      try {
        const compared = compare(decodePNG(chromeBytes), decodePNG(nativeBytes), { category: fixture.category });
        row.comparison = compared.report; row.verdict = compared.report.verdict;
        row.drawStatsEqual = JSON.stringify(chrome.draw.drawnLines) === JSON.stringify(row.native.draw.drawnLines) &&
          chrome.draw.opacityLayers === row.native.draw.opacityLayers && chrome.draw.opacityLayerFallbacks === row.native.draw.opacityLayerFallbacks;
        await writeFile(join(dir, "diff.png"), encodePNG(compared.diff));
        await writeFile(join(dir, "chrome-native-diff.png"), encodePNG(compared.sideBySide));
        row.images = ["chrome.png", "native.png", "diff.png", "chrome-native-diff.png"].map(name => `${key}/${name}`);
      } catch (error) { row.verdict = "COMPARISON_ERROR"; row.error = error.message; }
      await writeJSON(join(dir, "result.json"), row);
    }
    for (const probe of probes(resources)) {
      const dir = join(out, `probe-${probe.name}`); await mkdir(dir);
      const request = join(dir, "request.json"); await writeJSON(request, { scene: probe.scene, scale: probe.scale });
      const chrome = await capture(page, probe.scene, probe.scale);
      const actual = native ? await nativeCapture(native.binary, out, request, join(dir, "native.png")) : null;
      report.probes.push({ name: probe.name, expectedChromeCode: probe.expectedChromeCode, chrome, native: actual,
        chromePass: !chrome.ok && chrome.code === probe.expectedChromeCode, nativeRejected: actual ? !actual.ok : null });
    }
  });
  report.sourceHashesAfter = await sourceHashes(root);
  report.sourceChangedDuringRun = JSON.stringify(report.sourceHashesBefore) !== JSON.stringify(report.sourceHashesAfter);
  report.summary = { captures: report.cases.length, nontext: report.cases.filter(c => c.category === "nontext").length,
    text: report.cases.filter(c => c.category === "text").length,
    verdicts: Object.fromEntries([...new Set(report.cases.map(c => c.verdict))].map(v => [v, report.cases.filter(c => c.verdict === v).length])),
    probes: report.probes.length, chromeProbeFailures: report.probes.filter(p => !p.chromePass).length,
    nativeProbeFailures: native ? report.probes.filter(p => !p.nativeRejected).length : null,
    drawStatsMismatches: report.cases.filter(c => c.drawStatsEqual === false).length };
  if (native && (report.cases.some(c => ["FAIL", "NATIVE_ERROR", "COMPARISON_ERROR"].includes(c.verdict)) ||
      report.summary.nativeProbeFailures || report.summary.chromeProbeFailures || report.summary.drawStatsMismatches || report.sourceChangedDuringRun)) process.exitCode = 2;
} catch (error) {
  report.error = { message: error.message, stack: error.stack }; process.exitCode = 1;
} finally {
  await writeJSON(join(out, "report.json"), report);
  await writeJSON(join(here, "out/latest.json"), { path: relative(here, out) });
  console.log(JSON.stringify({ report: relative(root, join(out, "report.json")), summary: report.summary, error: report.error?.message }, null, 2));
}
