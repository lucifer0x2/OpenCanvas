import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { ROOT, OUT } from "./environment.mjs";

export async function startServer(fonts) {
  const files = new Map([
    ["/renderer.js", [`${ROOT}renderer/dist/index.js`, "text/javascript"]],
    ["/transform.js", [new URL("transform.js", OUT), "text/javascript"]],
    ["/oracle-browser.mjs", [new URL("browser-capture.mjs", import.meta.url), "text/javascript"]],
    ...fonts.map((f) => [f.manifest.url, [new URL(f.sourceBytes, OUT), "font/ttf"]]),
  ]);
  const failures = [];
  const server = createServer(async (req, res) => {
    const path = new URL(req.url, "http://localhost").pathname;
    if (path === "/") {
      res.writeHead(200, { "content-type": "text/html; charset=utf-8", "cache-control": "no-store" });
      return res.end('<!doctype html><html lang="und"><head><meta charset="utf-8"><link rel="icon" href="data:,"></head><body></body></html>');
    }
    const file = files.get(path);
    if (!file) { failures.push(`HTTP 404 ${path}`); res.writeHead(404); return res.end(); }
    try {
      const bytes = await readFile(file[0]);
      res.writeHead(200, { "content-type": file[1], "cache-control": "no-store" }); res.end(bytes);
    } catch (error) {
      failures.push(`HTTP 500 ${path}: ${error.code}`); res.writeHead(500); res.end();
    }
  });
  await new Promise((resolve, reject) => {
    server.once("error", reject); server.listen(0, "127.0.0.1", resolve);
  });
  return { url: `http://127.0.0.1:${server.address().port}`, failures,
    close: () => new Promise((resolve, reject) => {
      server.close((error) => error ? reject(error) : resolve()); server.closeAllConnections();
    }) };
}
