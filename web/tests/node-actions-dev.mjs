// Exercise the documented Vite proxy against the same temporary real backend.
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import { waitFor } from "../../spikes/e2e/protocol.mjs";

export async function verifyDevExports(browser, backend, projectId) {
  const vite = spawn(
    process.execPath,
    ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", "0"],
    {
      cwd: fileURLToPath(new URL("../", import.meta.url)),
      env: { ...process.env, OPENCANVAS_API_URL: backend },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  let output = "";
  vite.stdout.on("data", (data) => {
    output += data;
  });
  vite.stderr.on("data", (data) => {
    output += data;
  });
  let page, base;
  try {
    base = await waitFor(
      () => output.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0],
      "Vite ephemeral startup",
    );
    page = await browser.newPage({ viewport: { width: 1300, height: 900 } });
    await page.goto(
      `${base}/?project=${projectId}&page=page&frame=frame&node=box`,
    );
    await page
      .getByRole("heading", { name: "Export box updated", exact: true })
      .waitFor();
    const posted = page.waitForResponse(
      (r) => r.request().method() === "POST" && r.url().endsWith("/exports"),
    );
    const file = page.waitForResponse((r) => /\/files\/0$/.test(r.url()));
    await Promise.all([
      page.waitForEvent("download"),
      page.getByRole("button", { name: "Export PNG", exact: true }).click(),
    ]);
    assert.equal((await posted).status(), 200);
    const response = await file;
    assert.equal(
      new URL(response.url()).origin,
      base,
      "download remains on dev origin",
    );
    assert.equal(response.headers()["content-type"], "image/png");
    const foreign = await fetch(
      `${base}/api/v1/projects/${projectId}/exports`,
      {
        method: "POST",
        headers: {
          "Content-Type": "application/json",
          Origin: "https://evil.example",
        },
        body: JSON.stringify({ nodeIds: ["box"] }),
      },
    );
    assert.equal(
      foreign.status,
      403,
      "dev proxy preserves rejection of foreign origins",
    );
    console.log(
      "ok - real Vite proxy POST and PNG download succeed; foreign Origin stays rejected",
    );
  } finally {
    await page?.close();
    const exited = once(vite, "exit");
    vite.kill("SIGTERM");
    await exited;
    if (base) console.log(`cleaned temporary Vite port ${new URL(base).port}`);
  }
}
