// Finite weight × optical size × tracking capture; never overwrites prior reports.
import { readFile, writeFile } from "node:fs/promises";
import { OUT, PIN, prepareRenderer, playwrightRuntime, sha256 } from "./environment.mjs";
import { startServer } from "./http-server.mjs";
import { thresholdCases } from "./corpus.mjs";
import { eligibility } from "./schema.mjs";
import { variableFixtures, variableCases } from "./v1-fixtures.mjs";

const source = JSON.parse(await readFile(new URL("oracle.json", OUT)));
const fonts = await variableFixtures(source);
const sourceHashes = await prepareRenderer();
for (const name of ["v1-capture", "v1-fixtures"]) sourceHashes[`spikes/native-render-parity/${name}.mjs`] =
  sha256(await readFile(new URL(`${name}.mjs`, import.meta.url)));
const { requests, cases } = variableCases(fonts);
const { chromium } = await playwrightRuntime(), server = await startServer(fonts);
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
  }, fonts.map((f) => f.manifest));
  const advanceCases = requests.length;
  const result = await page.evaluate((r) => oracle.capture(r), requests);
  // Boundary subset covers all five Inter weights/sizes/tracking values, not an
  // unbounded stress sweep. All candidates preserve the browser's exact ceil64.
  const subset = requests.filter((r) => r.fontId.startsWith("v1-inter-") &&
    r.style === "normal" && cases[r.key].label === "root");
  const boundaries = thresholdCases(subset, result.measurements, cases);
  const boundary = await page.evaluate((r) => oracle.capture(r), boundaries);
  requests.push(...boundaries);
  Object.assign(result.measurements, boundary.measurements);
  Object.assign(result.probes, boundary.probes);
  if (server.failures.length) throw new Error(server.failures.join("\n"));
  const report = { ...source, generatedAt: new Date().toISOString(), fonts,
    environment: { ...source.environment, node: process.version, platform: process.platform,
      arch: process.arch, chromiumVersion: browser.version(), sfTrak: false, sourceHashes },
    corpus: { path: "spikes/native-render-parity/v1-capture.mjs", sha256:
      sourceHashes["spikes/native-render-parity/v1-capture.mjs"], advanceCases, wrapCases: boundaries.length },
    requests, cases, browser: { ...ready, ...result, eligibility: Object.fromEntries(
      requests.map((r) => [r.key, eligibility(result.measurements[r.key], cases[r.key])])) } };
  await writeFile(new URL("v1-oracle.json", OUT), JSON.stringify(report, null, 2));
  console.log(JSON.stringify({ requests: requests.length, advanceCases, boundaries: boundaries.length,
    fonts: fonts.length, output: "out/v1-oracle.json" }));
} finally {
  try { await browser?.close(); } finally { await server.close(); }
}
