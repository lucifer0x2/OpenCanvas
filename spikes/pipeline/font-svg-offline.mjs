// Validates the real SVG produced by the server integration test, in an offline new context.
import assert from "node:assert/strict";
import path from "node:path";
import { pathToFileURL } from "node:url";
process.env.PLAYWRIGHT_BROWSERS_PATH ??= path.resolve(".runtime/browsers");
const { chromium } = await import("playwright");
const browser = await chromium.launch({ headless: true });
try {
  const context = await browser.newContext({ offline: true });
  const page = await context.newPage();
  const network = [];
  page.on("request", (request) => {
    if (/^https?:/.test(request.url())) network.push(request.url());
  });
  await page.goto(pathToFileURL(process.argv[2]).href);
  const result = await page.evaluate(async () => {
    await document.fonts.ready;
    const text = [...document.querySelectorAll("text")];
    return {
      faces: [...document.fonts].map((font) => ({
        family: font.family,
        status: font.status,
      })),
      text: text.map((node) => ({
        copy: node.textContent,
        width: node.getComputedTextLength(),
        family: node.getAttribute("font-family"),
      })),
    };
  });
  assert.equal(result.faces.length, 2);
  assert.ok(
    result.faces.every(
      (font) =>
        font.status === "loaded" &&
        /^oc_[a-f0-9]{64}_400_normal$/.test(font.family),
    ),
  );
  assert.equal(result.text.length, 2);
  assert.equal(result.text[0].copy, "Tiếng Việt OpenCanvas");
  assert.equal(result.text[0].copy, result.text[1].copy);
  assert.notEqual(result.text[0].family, result.text[1].family);
  assert.ok(Math.abs(result.text[0].width - result.text[1].width) > 1);
  assert.deepEqual(network, []);
  console.log(JSON.stringify(result));
} finally {
  await browser.close();
}
