// Open real server exports in an offline, isolated Chromium context.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import path from "node:path";
import { pathToFileURL } from "node:url";
process.env.PLAYWRIGHT_BROWSERS_PATH ??= path.resolve(".runtime/browsers");
const { chromium } = await import("playwright");
const browser = await chromium.launch({ headless: true });
try {
  const context = await browser.newContext({ offline: true });
  const results = [];
  for (const [index, file] of process.argv.slice(2).entries()) {
    const page = await context.newPage();
    const network = [];
    page.on("request", (request) => {
      if (/^https?:/.test(request.url())) network.push(request.url());
    });
    await page.goto(pathToFileURL(file).href);
    const result = await page.evaluate(async () => {
      await document.fonts.ready;
      const text = document.querySelector("text");
      return {
        faces: [...document.fonts].map((f) => ({ family: f.family, status: f.status })),
        copy: text.textContent,
        width: text.getComputedTextLength(),
      };
    });
    assert.equal(result.faces.length, 1);
    const suffix = ["400_normal", "700_normal", "400_italic"][index];
    assert.match(result.faces[0].family, new RegExp(`^oc_[a-f0-9]{64}_${suffix}$`));
    assert.equal(result.faces[0].status, "loaded");
    assert.equal(result.copy, "Tiếng Việt OpenCanvas");
    assert.ok(result.width > 100);
    assert.deepEqual(network, []);
    result.pixels = createHash("sha256").update(await page.screenshot()).digest("hex");
    results.push(result);
    await page.close();
  }
  assert.equal(results.length, 3);
  assert.equal(new Set(results.map((r) => r.pixels)).size, 3);
  assert.ok(Math.abs(results[0].width - results[1].width) > 0.1);
  console.log(JSON.stringify(results));
} finally {
  await browser.close();
}
