// Dev-only capture from the exact existing G1 bytes. Does not replace G1/G2 files.
import { readFile, writeFile } from "node:fs/promises";
import { OUT, PIN, prepareRenderer, playwrightRuntime, sha256 } from "./environment.mjs";
import { startServer } from "./http-server.mjs";
import { thresholdCases } from "./corpus.mjs";
import { eligibility } from "./schema.mjs";
import { scriptCases } from "./h1-cases.mjs";

const source = JSON.parse(await readFile(new URL("oracle.json", OUT)));
const sourceHashes = await prepareRenderer();
for (const file of ["h1-cases.mjs", "h1-probe.mjs"])
  sourceHashes[`spikes/native-render-parity/${file}`] = sha256(await readFile(new URL(file, import.meta.url)));
const requests = [], cases = {};
for (const { manifest: m } of source.fonts) {
  for (const fontSize of [12, 16, 32]) {
    for (const letterSpacing of [0, -2, 0.5]) {
      for (const [label, text] of Object.entries(scriptCases)) {
        const key = `current/h1/${m.id}/${fontSize}/${letterSpacing}/${label}`;
        requests.push({ key, text, fontId: m.id, fontHash: m.hash, family: m.family,
          weight: m.weight, style: m.style, fontSize, lineHeight: fontSize * 1.375,
          letterSpacing, textTransform: "none", width: null });
        cases[key] = { group: "script-mark-stress", label, declaredFallback: false, thresholdOf: null };
      }
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
    corpus: { path: "spikes/native-render-parity/h1-cases.mjs", sha256:
      sourceHashes["spikes/native-render-parity/h1-cases.mjs"], advanceCases, wrapCases: thresholds.length },
    requests, cases, browser: { ...ready, ...result, eligibility: Object.fromEntries(
      requests.map((r) => [r.key, eligibility(result.measurements[r.key], cases[r.key])])) } };
  await writeFile(new URL("h1-oracle.json", OUT), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ requests: requests.length, advanceCases, thresholds: thresholds.length,
    fonts: source.fonts.length, output: "out/h1-oracle.json" }));
} finally {
  try { await browser?.close(); } finally { await server.close(); }
}
