import { createServer } from "node:http";
import { readFile, readdir } from "node:fs/promises";
import { join, resolve } from "node:path";
import { build } from "esbuild";
import { hash } from "./scene.mjs";
export async function serve(routes) {
  const server = createServer((request, response) => {
    const item = routes.get(request.url);
    if (!item) { response.writeHead(404).end(); return; }
    response.writeHead(200, { "content-type": item.type, "cache-control": "no-store" }).end(item.bytes);
  });
  await new Promise((ok, fail) => { server.once("error", fail); server.listen(0, "127.0.0.1", ok); });
  return { origin: `http://127.0.0.1:${server.address().port}`,
    close: () => new Promise((ok, fail) => { server.close(error => error ? fail(error) : ok()); server.closeAllConnections(); }) };
}
export async function withOracle(root, out, action) {
  process.env.PLAYWRIGHT_BROWSERS_PATH = join(root, ".runtime/browsers");
  const { chromium } = await import("playwright");
  const playwrightVersion = JSON.parse(await readFile(new URL("../../node_modules/playwright/package.json", import.meta.url))).version;
  if (playwrightVersion !== "1.64.1") throw Error(`Unpinned Playwright ${playwrightVersion}`);
  const bundle = join(out, "renderer.js");
  await build({ entryPoints: [join(root, "renderer/index.ts")], bundle: true, format: "esm", outfile: bundle, logLevel: "silent" });
  const bytes = await readFile(bundle), routes = new Map([
    ["/", { type: "text/html", bytes: Buffer.from('<!doctype html><canvas id="target"></canvas>') }],
    ["/renderer.js", { type: "text/javascript", bytes }],
  ]);
  const server = await serve(routes); let browser;
  try {
    browser = await chromium.launch({ headless: true, args: ["--force-color-profile=srgb"] });
    if (browser.version() !== "156.0.8078.4") throw Error(`Unpinned Chromium ${browser.version()}`);
    const context = await browser.newContext({ deviceScaleFactor: 1 });
    await context.route("**/*", route => new URL(route.request().url()).origin === server.origin ? route.continue() : route.abort());
    const page = await context.newPage(); await page.goto(server.origin);
    await page.evaluate(async () => { window.renderer = await import("/renderer.js"); });
    if (await page.evaluate(() => window.renderer.SCENE_VERSION !== "scene-3" || window.renderer.RENDERER_VERSION !== "web-4")) throw Error("Renderer pipeline changed");
    return await action({ page, browserVersion: browser.version(), playwrightVersion,
      rendererBundleHash: hash(bytes), browserExecutable: resolve(chromium.executablePath()),
      async refreshResources() {
        for (const name of await readdir(join(out, "resources"))) routes.set(`/resources/${name}`, {
          type: "application/octet-stream", bytes: await readFile(join(out, "resources", name)),
        });
      } });
  } finally { try { await browser?.close(); } finally { await server.close(); } }
}
export async function capture(page, scene, scale) {
  return page.evaluate(async ({ scene, scale }) => {
    const resources = await window.renderer.loadResources(scene.fonts, scene.assets, { strict: false, timeoutMs: 5000 });
    try {
      const draw = window.renderer.drawScene(document.querySelector("canvas"), scene, resources, { scale, strict: true });
      return { ok: true, draw, readiness: resources.readiness, diagnostics: resources.diagnostics,
        png: document.querySelector("canvas").toDataURL("image/png").split(",")[1] };
    } catch (error) {
      return { ok: false, code: error.code ?? "UNEXPECTED_ERROR", message: error.message,
        readiness: resources.readiness, diagnostics: resources.diagnostics };
    }
  }, { scene, scale });
}
