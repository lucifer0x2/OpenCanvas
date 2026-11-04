// Supplemental controlled probes, preserving the primary G1 capture unchanged.
import { readFile, writeFile, unlink, rename } from "node:fs/promises";
import { OUT, PIN, prepareRenderer, playwrightRuntime, sha256 } from "./environment.mjs";
import { startServer } from "./http-server.mjs";
import { thresholdCases } from "./corpus.mjs";
import { eligibility } from "./schema.mjs";

async function main() {
  const output = new URL("probe-oracle.json", OUT), pending = new URL("probe-oracle.json.tmp", OUT);
  const remove = (path) => unlink(path).catch((error) => { if (error.code !== "ENOENT") throw error; });
  await remove(output);
  const source = JSON.parse(await readFile(new URL("oracle.json", OUT)));
  const sourceHashes = await prepareRenderer();
  sourceHashes["spikes/native-render-parity/probe.mjs"] = sha256(await readFile(new URL(import.meta.url)));
  const requests = [], cases = {};
  for (const { manifest: m } of source.fonts) {
    for (const fontSize of [12, 14, 16, 17.5, 24, 32]) {
      for (const letterSpacing of [0, -2, 0.5]) {
        for (const [label, text] of Object.entries({ single: "n", latin: "AV To office ffi",
          vietnamese: "Tiếng Việt ắ", mixed: "ς i̇", invisible: "A\u2060B\u200bC" })) {
          const key = `current/probe/${m.id}/${fontSize}/${letterSpacing}/${label}`;
          requests.push({ key, text, fontId: m.id, fontHash: m.hash, family: m.family,
            weight: m.weight, style: m.style, fontSize, lineHeight: fontSize * 1.375,
            letterSpacing, textTransform: "none", width: null });
          cases[key] = { group: "controlled-optical-tracking", label,
            declaredFallback: false, thresholdOf: null };
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
    const failures = server.failures;
    page.on("pageerror", (error) => failures.push(error.message));
    page.on("requestfailed", (request) => failures.push(request.url()));
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
    if (failures.length) throw new Error(failures.join("\n"));
    const report = { ...source, generatedAt: new Date().toISOString(),
      environment: { ...source.environment, node: process.version, platform: process.platform,
        arch: process.arch, chromiumVersion: browser.version(), sourceHashes },
      corpus: { path: "spikes/native-render-parity/probe.mjs", sha256:
        sourceHashes["spikes/native-render-parity/probe.mjs"], advanceCases, wrapCases: thresholds.length },
      requests, cases, browser: { ...ready, ...result, eligibility: Object.fromEntries(
        requests.map((r) => [r.key, eligibility(result.measurements[r.key], cases[r.key])])) } };
    await writeFile(pending, JSON.stringify(report, null, 2));
    await rename(pending, output);
    console.log(JSON.stringify({ requests: requests.length, fonts: source.fonts.length,
      output: "out/probe-oracle.json" }));
  } finally {
    try { await browser?.close(); } finally {
      try { await server.close(); } finally { await remove(pending); }
    }
  }
}
await main();
