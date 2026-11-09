import { readFile, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { join } from "node:path";
import { ROOT, OUT } from "./runtime.mjs";

export async function preview(base, fixtures) {
  const require = createRequire(`${ROOT}web/package.json`);
  const { createServer } = await import(require.resolve("vite"));
  const probe = await readFile(
    new URL("./browser-probe.mjs", import.meta.url),
    "utf8",
  );
  const results = [];
  const observer = await readFile(
    new URL("./browser-observe.mjs", import.meta.url),
    "utf8",
  );
  let completed;
  const browserComplete = new Promise((resolve) => {
    completed = resolve;
  });
  const server = await createServer({
    root: `${ROOT}web`,
    configFile: false,
    server: {
      host: "127.0.0.1",
      port: Number(process.env.STRESS_PREVIEW_PORT ?? 0),
      strictPort: true,
      fs: { allow: [ROOT] },
      proxy: {
        "/api": {
          target: base,
          changeOrigin: true,
          configure(proxy) {
            proxy.on("proxyReq", (outgoing, incoming) => {
              if (incoming.headers.origin === `http://${incoming.headers.host}`)
                outgoing.setHeader("Origin", base);
            });
          },
        },
      },
    },
    plugins: [
      {
        name: "isolated-all-stress",
        configureServer(vite) {
          vite.middlewares.use(async (req, res, next) => {
            if (req.url === "/__stress/browser-observe.mjs")
              return res
                .writeHead(200, { "content-type": "text/javascript" })
                .end(observer);
            if (req.url === "/__stress/probe.mjs")
              return res
                .writeHead(200, { "content-type": "text/javascript" })
                .end(probe);
            if (req.url !== "/__stress/result") return next();
            let bytes = "";
            for await (const chunk of req) bytes += chunk;
            const value = JSON.parse(bytes);
            results.push(value);
            await writeFile(
              join(OUT, "browser.json"),
              JSON.stringify(results, null, 2),
            );
            console.log(
              `BROWSER_RESULT ${value.profile} ${value.error ?? `cold=${value.cold.readyMs.toFixed(1)}ms`}`,
            );
            if (results.length === 4 || value.error) completed(results);
            res
              .writeHead(200, { "content-type": "application/json" })
              .end('{"saved":true}');
          });
        },
        transformIndexHtml(html, context) {
          const index = Math.max(
            0,
            Math.min(
              3,
              Number(
                new URL(
                  context.originalUrl ?? "/",
                  "http://stress",
                ).searchParams.get("profile") ?? 0,
              ),
            ),
          );
          const fixture = fixtures[index],
            next = index < 3 ? `/?profile=${index + 1}&scene=wasm` : null;
          return {
            html,
            tags: [
              {
                tag: "script",
                injectTo: "head-prepend",
                children: `window.__stressConfig=${JSON.stringify({ fixture, next })};localStorage.setItem('opencanvas.selection',JSON.stringify(${JSON.stringify({ projectId: fixture.projectId, pageId: "page", frameId: ":all:", nodeId: "" })}));`,
              },
              {
                tag: "script",
                attrs: { type: "module", src: "/__stress/probe.mjs" },
                injectTo: "head",
              },
            ],
          };
        },
      },
    ],
  });
  await server.listen();
  return {
    url: `http://127.0.0.1:${server.httpServer.address().port}/?profile=0&scene=wasm`,
    stop: () => server.close(),
    browserComplete,
  };
}
