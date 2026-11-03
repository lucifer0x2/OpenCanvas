import { spawn, execFileSync } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import { startContinuityFixture } from "./canvas-continuity-fixture.mjs";

process.env.PLAYWRIGHT_BROWSERS_PATH ??= fileURLToPath(
  new URL("../../.runtime/browsers", import.meta.url),
);

export async function withContinuityViewer(run) {
  const fixture = await startContinuityFixture();
  const vite = spawn(
    process.execPath,
    ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", "0"],
    {
      cwd: fileURLToPath(new URL("../", import.meta.url)),
      env: { ...process.env, OPENCANVAS_API_URL: fixture.baseURL },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  let output = "",
    browser;
  vite.stdout.on("data", (chunk) => (output += chunk));
  vite.stderr.on("data", (chunk) => (output += chunk));
  try {
    for (let i = 0; i < 100 && !output.match(/http:\/\/127\.0\.0\.1:\d+/); i++)
      await new Promise((resolve) => setTimeout(resolve, 50));
    const base = output.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0];
    if (!base) throw new Error(`Vite startup failed: ${output}`);
    const { chromium } = await import("playwright");
    browser = await chromium.launch({ headless: true });
    const page = await browser.newPage({
      viewport: { width: 1440, height: 1000 },
    });
    page.setDefaultTimeout(10_000);
    const errors = [];
    page.on("pageerror", (error) => {
      errors.push(error.message);
      if (process.env.CONTINUITY_BASELINE) console.error(error.message);
    });
    if (process.env.CONTINUITY_BASELINE)
      page.on("console", (message) => {
        if (message.type() === "error") console.error(message.text());
      });
    // Optional regression proof: replace only the hook with its committed pre-fix version.
    if (process.env.CONTINUITY_BASELINE) {
      const { transform } = await import("esbuild");
      const source = execFileSync(
        "git",
        ["show", "HEAD:web/src/hooks/useScene.ts"],
        { encoding: "utf8" },
      );
      const { code } = await transform(source, { loader: "ts", format: "esm" });
      await page.route("**/src/hooks/useScene.ts", async (route) => {
        const current = await (await route.fetch()).text();
        const react = current.match(/from "([^"]*\/react\.js[^"]*)"/)?.[1];
        if (!react)
          throw new Error("Cannot resolve Vite React import for baseline");
        const body = code
          .replace(
            /import \{([^}]+)\} from "react";/,
            (_, names) =>
              `import __react from ${JSON.stringify(react)}; const {${names}} = __react;`,
          )
          .replace(
            '"../../../renderer/index"',
            JSON.stringify(
              `/@fs${fileURLToPath(new URL("../../renderer/index.ts", import.meta.url))}`,
            ),
          )
          .replace(/"\.\.\/(sceneJob|state|diagnostics)"/g, '"/src/$1.ts"');
        await route.fulfill({ contentType: "text/javascript", body });
      });
    }
    await page.goto(base);
    await page.getByText("Scene r7", { exact: true }).waitFor();
    await page.waitForFunction(() => window.Konva?.stages?.length > 0);
    return await run(page, fixture, errors);
  } finally {
    await browser?.close();
    const exited = once(vite, "exit");
    vite.kill("SIGTERM");
    await exited;
    await fixture.close();
  }
}

export const camera = (page) =>
  page.evaluate(() => {
    const stage = window.Konva.stages[0];
    return { x: stage.x(), y: stage.y(), scale: stage.scaleX() };
  });
export const settled = (page) =>
  page.evaluate(
    () =>
      new Promise((resolve) =>
        requestAnimationFrame(() => requestAnimationFrame(resolve)),
      ),
  );
