// Starts the viewer for tests/zoom.mjs: an in-memory fixture API behind a vite dev server, and
// pages with the in-page zoom probe installed. Nothing touches 3847 or ~/.opencanvas.
import { spawn } from "node:child_process";
import { fileURLToPath } from "node:url";
import { startFixture } from "./fixture.mjs";
import { zoomProbe } from "./zoom-probe.mjs";

const web = fileURLToPath(new URL("../", import.meta.url));
const PORT = 5181;

/** Runs `run` while the viewer serves fixture scene `scene` ("light", "dense" or "stress"). */
export async function withViewer(scene, run) {
  const fixture = await startFixture({ scene });
  const vite = spawn(
    process.execPath,
    [
      "node_modules/vite/bin/vite.js",
      "--host",
      "127.0.0.1",
      "--port",
      String(PORT),
      "--strictPort",
    ],
    {
      cwd: web,
      env: { ...process.env, OPENCANVAS_API_URL: fixture.baseURL },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  let output = "";
  vite.stdout.on("data", (data) => (output += data));
  vite.stderr.on("data", (data) => (output += data));
  try {
    for (let attempt = 0; attempt < 100; attempt++) {
      try {
        if ((await fetch(`http://127.0.0.1:${PORT}`)).ok) break;
      } catch {}
      await new Promise((resolve) => setTimeout(resolve, 100));
    }
    return await run();
  } catch (error) {
    console.error(output);
    throw error;
  } finally {
    vite.kill("SIGTERM");
    await new Promise((resolve) => vite.once("exit", resolve));
    await fixture.close();
  }
}

/** A viewer page at `dpr`, ready on the fixture scene, and the scene URL it requested. */
export async function open(browser, dpr) {
  const page = await browser.newPage({
    viewport: { width: 1440, height: 1000 },
    deviceScaleFactor: dpr,
  });
  const errors = [];
  let sceneUrl;
  page.on("pageerror", (error) => errors.push(error.message));
  page.on("request", (request) => {
    if (request.url().includes("/scene?"))
      sceneUrl =
        new URL(request.url()).pathname + new URL(request.url()).search;
  });
  await page.addInitScript(zoomProbe);
  await page.goto(`http://127.0.0.1:${PORT}`);
  await page
    .getByText("Scene r7", { exact: true })
    .waitFor({ timeout: 30_000 });
  await page.waitForFunction(() => window.Konva?.stages?.length > 0);
  await page.evaluate(() => window.__zoom.frame());
  return { page, errors, sceneUrl: () => sceneUrl };
}
