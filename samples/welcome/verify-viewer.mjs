import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { fileURLToPath } from "node:url";
import { assertReadOnly, compareViewer, openViewer } from "../../spikes/e2e/browser.mjs";

const artifacts = new URL("../../artifacts/sample/", import.meta.url);
process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(new URL("../../.runtime/browsers/", import.meta.url));
const { chromium } = await import("playwright");
const summary = JSON.parse(await readFile(new URL("summary.json", artifacts), "utf8"));
const job = JSON.parse(await readFile(new URL("job.json", artifacts), "utf8"));
const browser = await chromium.launch({ headless: true, args: ["--no-sandbox"] });
try {
  const view = await openViewer(browser, summary.viewerUrl);
  await view.page.setViewportSize({ width: 1600, height: 1000 });
  await view.page.getByLabel("Project", { exact: true }).selectOption(summary.projectId);
  await view.page.getByLabel("Frame", { exact: true }).selectOption(summary.frameId);
  const pixels = await compareViewer(view, job);
  await view.page.getByRole("button", { name: "Fit frame", exact: true }).click();
  await view.page.screenshot({ path: fileURLToPath(new URL("viewer.png", artifacts)) });
  await view.page.getByRole("button", { name: "greeting", exact: true }).click();
  await view.page.getByRole("heading", { name: "greeting", exact: true }).waitFor();
  assertReadOnly(view);
  const result = {
    projectId: summary.projectId, revision: summary.revision, pixels,
    pageErrors: view.errors, viewerApiMethods: [...new Set(view.requests.map((request) => request.method))],
    selectedFrame: await view.page.getByLabel("Frame", { exact: true }).inputValue(),
    inspectorReadback: "greeting selected and inspected successfully",
  };
  assert.equal(result.selectedFrame, summary.frameId);
  await writeFile(new URL("verification.json", artifacts), JSON.stringify(result, null, 2));
  console.log(JSON.stringify(result, null, 2));
} finally {
  await browser.close();
}
