// Bounded regressions derived only from H1's script-mark and negative-advance bugs.
// Never replaces an H1/G2/root capture; exact original font bytes remain mandatory.
import { readFile, writeFile } from "node:fs/promises";
import { OUT, PIN, prepareRenderer, playwrightRuntime, sha256 } from "./environment.mjs";
import { startServer } from "./http-server.mjs";
import { thresholdCases } from "./corpus.mjs";
import { eligibility } from "./schema.mjs";

const samples = {
  crossScript: "a҃", sameScript: "Ж҃", inherited: "ȧ", stacked: "ắ",
  crossThenInherited: "a҃̇", wrapAtomic: "a҃a҃", negativeInk: "̇́",
};
const source = JSON.parse(await readFile(new URL("oracle.json", OUT)));
const sourceHashes = await prepareRenderer();
sourceHashes["spikes/native-render-parity/h2-probe.mjs"] =
  sha256(await readFile(new URL("h2-probe.mjs", import.meta.url)));
const requests = [], cases = {};
for (const { manifest: m } of source.fonts) {
  for (const letterSpacing of [0, -2, 0.5]) {
    for (const [label, text] of Object.entries(samples)) {
      const key = `current/h2/${m.id}/${letterSpacing}/${label}`;
      requests.push({ key, text, fontId: m.id, fontHash: m.hash, family: m.family,
        weight: m.weight, style: m.style, fontSize: 16, lineHeight: 22,
        letterSpacing, textTransform: "none", width: null });
      cases[key] = { group: "h2-targeted-regressions", label, declaredFallback: false, thresholdOf: null };
    }
  }
}
const { chromium } = await playwrightRuntime(), server = await startServer(source.fonts);
let browser;
try {
  browser = await chromium.launch({ headless: true });
  if (browser.version() !== PIN.chromium) throw new Error("Chromium pin mismatch");
  const page = await browser.newPage({ locale: "en-US", deviceScaleFactor: 1 });
  page.on("pageerror", (error) => server.failures.push(error.message));
  page.on("requestfailed", (request) => server.failures.push(request.url()));
  await page.goto(server.url, { waitUntil: "load" });
  const ready = await page.evaluate(async (fonts) => {
    globalThis.oracle = await import("/oracle-browser.mjs");
    return oracle.initialize(fonts);
  }, source.fonts.map((f) => f.manifest));
  const advanceCases = requests.length;
  const result = await page.evaluate((r) => oracle.capture(r), requests);
  const thresholds = thresholdCases(requests, result.measurements, cases);
  const boundary = await page.evaluate((r) => oracle.capture(r), thresholds);
  requests.push(...thresholds);
  Object.assign(result.measurements, boundary.measurements);
  Object.assign(result.probes, boundary.probes);
  if (server.failures.length) throw new Error(server.failures.join("\n"));
  const report = { ...source, generatedAt: new Date().toISOString(),
    environment: { ...source.environment, node: process.version, platform: process.platform,
      arch: process.arch, chromiumVersion: browser.version(), sourceHashes },
    corpus: { path: "spikes/native-render-parity/h2-probe.mjs", sha256:
      sourceHashes["spikes/native-render-parity/h2-probe.mjs"], advanceCases, wrapCases: thresholds.length },
    requests, cases, browser: { ...ready, ...result, eligibility: Object.fromEntries(
      requests.map((r) => [r.key, eligibility(result.measurements[r.key], cases[r.key])])) } };
  await writeFile(new URL("h2-oracle.json", OUT), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ requests: requests.length, advanceCases, thresholds: thresholds.length,
    fonts: source.fonts.length, output: "out/h2-oracle.json" }));
} finally {
  try { await browser?.close(); } finally { await server.close(); }
}
