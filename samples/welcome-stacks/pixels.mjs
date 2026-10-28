import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { THRESHOLD } from "./design.mjs";

export async function compareImages(artifacts) {
  process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(new URL("../../.runtime/browsers/", import.meta.url));
  const { chromium } = await import("playwright");
  const browser = await chromium.launch({ headless: true, args: ["--no-sandbox"] });
  try {
    const page = await browser.newPage();
    const dataUrl = async (url) => `data:image/png;base64,${(await readFile(url)).toString("base64")}`;
    const result = await page.evaluate(async ({ baseline, stacks, historical }) => {
      const decode = async (url) => {
        const image = new Image(); image.src = url; await image.decode();
        const canvas = document.createElement("canvas");
        canvas.width = image.width; canvas.height = image.height;
        const context = canvas.getContext("2d"); context.drawImage(image, 0, 0);
        return { data: context.getImageData(0, 0, image.width, image.height).data, canvas };
      };
      const a = await decode(baseline), b = await decode(stacks), c = await decode(historical);
      const diff = document.createElement("canvas"); diff.width = a.canvas.width; diff.height = a.canvas.height;
      const ctx = diff.getContext("2d"), output = ctx.createImageData(diff.width, diff.height);
      let changedPixels = 0, absoluteError = 0, historicalChangedChannels = 0;
      for (let i = 0; i < a.data.length; i += 4) {
        let changed = false;
        for (let j = 0; j < 4; j++) {
          const error = Math.abs(a.data[i + j] - b.data[i + j]);
          absoluteError += error; changed ||= error > 0;
          historicalChangedChannels += Number(a.data[i + j] !== c.data[i + j]);
        }
        changedPixels += Number(changed);
        output.data.set(changed ? [255, 0, 100, 255] : [245, 246, 250, 255], i);
      }
      ctx.putImageData(output, 0, 0);
      return { width: diff.width, height: diff.height, changedPixels,
        changedPixelRatio: changedPixels / (diff.width * diff.height),
        meanAbsoluteChannelError: absoluteError / a.data.length,
        historicalChangedChannels, diff: diff.toDataURL("image/png") };
    }, {
      baseline: await dataUrl(new URL("baseline-1040.png", artifacts)),
      stacks: await dataUrl(new URL("stacks-1040.png", artifacts)),
      historical: await dataUrl(new URL("../../artifacts/sample/dashboard.png", import.meta.url)),
    });
    await writeFile(new URL("desktop-diff.png", artifacts), Buffer.from(result.diff.split(",")[1], "base64"));
    delete result.diff;
    result.threshold = THRESHOLD;
    await writeFile(new URL("comparison.json", artifacts), JSON.stringify(result, null, 2));
    assert.ok(result.changedPixelRatio <= THRESHOLD.changedPixelRatio, JSON.stringify(result));
    assert.ok(result.meanAbsoluteChannelError <= THRESHOLD.meanAbsoluteChannelError, JSON.stringify(result));
    return result;
  } finally { await browser.close(); }
}
