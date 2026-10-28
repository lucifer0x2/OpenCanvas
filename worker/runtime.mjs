import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import path from "node:path";
import { fileURLToPath } from "node:url";

export const root = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "..",
);

/** Local bundle server exposes a fixed allowlist, never the workspace/project filesystem. */
export async function startRuntime({ resourceHandler, executablePath } = {}) {
  const files = new Map([
    ["/renderer/dist/index.js", ["renderer/dist/index.js", "text/javascript"]],
    [
      "/renderer/fonts/NotoSans.ttf",
      ["renderer/fonts/NotoSans.ttf", "font/ttf"],
    ],
  ]);
  const server = createServer(async (req, res) => {
    try {
      if (resourceHandler && (await resourceHandler(req, res))) return;
      const pathname = new URL(req.url, "http://localhost").pathname;
      if (pathname === "/") {
        res.writeHead(200, { "content-type": "text/html" });
        res.end(
          '<!doctype html><meta charset="utf-8"><canvas id="scene"></canvas>',
        );
        return;
      }
      const file = files.get(pathname);
      if (!file) {
        res.writeHead(404);
        res.end();
        return;
      }
      const body = await readFile(path.join(root, file[0]));
      res.writeHead(200, {
        "content-type": file[1],
        "access-control-allow-origin": "*",
      });
      res.end(body);
    } catch {
      res.writeHead(500);
      res.end();
    }
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject);
    server.listen(0, "127.0.0.1", resolve);
  });
  const baseURL = `http://127.0.0.1:${server.address().port}`;
  process.env.PLAYWRIGHT_BROWSERS_PATH ??= path.join(root, ".runtime/browsers");
  let browser;
  try {
    const { chromium } = await import("playwright");
    browser = await chromium.launch({
      headless: true,
      executablePath,
      timeout: 15_000,
    });
    const page = await browser.newPage({
      viewport: { width: 800, height: 600 },
      deviceScaleFactor: 1,
    });
    await page.goto(baseURL, { timeout: 10_000 });
    await page.evaluate(async () => {
      globalThis.renderer = await import("/renderer/dist/index.js");
    });
    return {
      page,
      browser,
      baseURL,
      browserVersion: browser.version(),
      close: async () => {
        await browser.close();
        await new Promise((resolve) => server.close(resolve));
      },
    };
  } catch (error) {
    await browser?.close();
    server.close();
    throw error;
  }
}
