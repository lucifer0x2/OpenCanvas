import { mkdir, writeFile, rename, unlink } from "node:fs/promises";
import { OUT, PIN, prepareRenderer, playwrightRuntime } from "./environment.mjs";
import { prepareFonts } from "./font-fixtures.mjs";
import { buildCorpus, thresholdCases } from "./corpus.mjs";
import { startServer } from "./http-server.mjs";
import { REPORT_SCHEMA, eligibility } from "./schema.mjs";

async function main() {
  const args = process.argv.slice(2);
  if (args.some((arg) => arg !== "--sf-trak")) throw new Error("Usage: node spikes/native-render-parity/capture.mjs [--sf-trak]");
  await mkdir(OUT, { recursive: true });
  // A failed rerun must not leave an earlier report looking like this run's evidence.
  await unlink(new URL("oracle.json", OUT)).catch((error) => { if (error.code !== "ENOENT") throw error; });
  const sourceHashes = await prepareRenderer();
  const { chromium } = await playwrightRuntime();
  const sfTrak = args.includes("--sf-trak");
  const fonts = await prepareFonts({ sfTrak });
  const corpus = await buildCorpus(fonts);
  const server = await startServer(fonts);
  let browser;
  const failures = server.failures;
  try {
    browser = await chromium.launch({ headless: true });
    const version = browser.version();
    if (version !== PIN.chromium) throw new Error(`Unexpected Chromium ${version}; expected ${PIN.chromium}`);
    const page = await browser.newPage({ viewport: { width: 800, height: 600 }, deviceScaleFactor: 1, locale: "en-US" });
    page.setDefaultTimeout(30_000);
    page.on("requestfailed", (request) => failures.push(`Request failed: ${new URL(request.url()).pathname}`));
    page.on("response", (response) => { if (!response.ok()) failures.push(`HTTP ${response.status()} ${new URL(response.url()).pathname}`); });
    page.on("pageerror", (error) => failures.push(`Page error: ${error.message}`));
    await page.goto(server.url, { waitUntil: "load" });
    const ready = await page.evaluate(async (fonts) => {
      globalThis.oracle = await import("/oracle-browser.mjs");
      return oracle.initialize(fonts);
    }, fonts.map((f) => f.manifest));
    if (ready.rendererVersion !== "web-4" || ready.sceneVersion !== "scene-3") throw new Error("Renderer semantics version mismatch");
    const result = await page.evaluate((requests) => oracle.capture(requests), corpus.requests);
    const thresholds = thresholdCases(corpus.requests, result.measurements, corpus.cases);
    const boundaryResult = await page.evaluate((requests) => oracle.capture(requests), thresholds);
    Object.assign(result.measurements, boundaryResult.measurements);
    Object.assign(result.probes, boundaryResult.probes);
    corpus.requests.push(...thresholds);
    if (failures.length) throw new Error(failures.join("\n"));
    const policy = Object.fromEntries(corpus.requests.map((r) => [r.key, eligibility(result.measurements[r.key], corpus.cases[r.key])]));
    const report = { schemaVersion: 1, kind: "chromium-measurement-oracle-only", generatedAt: new Date().toISOString(),
      environment: { node: process.version, platform: process.platform, arch: process.arch,
        playwrightVersion: PIN.playwright, chromiumVersion: version, chromiumRevision: PIN.revision,
        headless: true, args: [], sfTrak, sourceHashes }, fonts, corpus: corpus.source, requests: corpus.requests,
      cases: corpus.cases, browser: { ...ready, ...result, eligibility: policy } };
    await writeFile(new URL("report-schema.json", OUT), JSON.stringify(REPORT_SCHEMA, null, 2) + "\n");
    await writeFile(new URL("oracle.json.tmp", OUT), JSON.stringify(report, null, 2) + "\n");
    await rename(new URL("oracle.json.tmp", OUT), new URL("oracle.json", OUT));
    console.log(JSON.stringify({ output: "spikes/native-render-parity/out/oracle.json", cases: corpus.requests.length,
      eligible: Object.values(policy).filter((p) => p.status === "eligible").length,
      excluded: Object.values(policy).filter((p) => p.status === "excluded").length,
      fonts: fonts.map((f) => ({ id: f.manifest.id, hash: f.manifest.hash, trak: f.tableTags.includes("trak") })),
      chromium: version, nativeCompared: false }, null, 2));
  } finally {
    try { await browser?.close(); } finally { await server.close(); }
    await unlink(new URL("oracle.json.tmp", OUT)).catch((error) => { if (error.code !== "ENOENT") throw error; });
  }
}

main().catch((error) => { console.error(error); process.exitCode = 1; });
