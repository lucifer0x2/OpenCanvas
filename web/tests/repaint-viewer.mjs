// Production build of a viewer tree with test-only counters, served with an API proxy.
// The counters wrap module exports through a build-time alias, so product code carries no probe.
import { mkdir, writeFile } from "node:fs/promises";
import { createRequire } from "node:module";
import { join } from "node:path";

const COUNTS =
  "(globalThis.__repaintCounts ??= { paint: {}, paintTotal: 0, paintMs: 0, index: 0, inPaint: 0, publish: 0 })";
/** Wrappers by module: paint/index calls per frame, and page view publications (setLoaded). */
const WRAPPERS = {
  "renderer/index.ts": (from) => `import * as R from ${from};
export * from ${from};
export function paintScene(context, scene, ...rest) {
  const p = ${COUNTS}, id = scene.provenance.frameId, t = performance.now();
  p.paintTotal++; p.paint[id] = (p.paint[id] ?? 0) + 1; p.inPaint++;
  try { return R.paintScene(context, scene, ...rest); }
  finally { p.inPaint--; p.paintMs += performance.now() - t; }
}
export function indexScene(...args) { ${COUNTS}.index++; return R.indexScene(...args); }
`,
  // usePageScenes publishes with an explicit \`finished\` option; the render-time transition does not.
  "web/src/scene/staleFrames.ts": (from) => `import * as S from ${from};
export * from ${from};
export function pageView(entries, options) {
  if ("finished" in options) ${COUNTS}.publish++;
  return S.pageView(entries, options);
}
`,
};

/** `root` is a repository checkout (this one or a pristine worktree); `out` a scratch dir. */
export async function buildViewer(root, out) {
  const require = createRequire(join(root, "web/package.json"));
  const { build } = await import(require.resolve("vite"));
  await mkdir(out, { recursive: true });
  const aliases = new Map();
  for (const [path, source] of Object.entries(WRAPPERS)) {
    const wrapper = join(out, `${path.replaceAll("/", "-")}.probe.mjs`);
    await writeFile(wrapper, source(JSON.stringify(join(root, path))));
    aliases.set(join(root, path), wrapper);
  }
  const wrappers = new Set(aliases.values());
  await build({
    root: join(root, "web"),
    configFile: join(root, "web/vite.config.ts"),
    logLevel: "warn",
    build: { outDir: join(out, "dist"), emptyOutDir: true },
    plugins: [
      {
        name: "repaint-probe",
        enforce: "pre",
        async resolveId(source, importer, options) {
          if (!importer || wrappers.has(importer) || !/(renderer\/index|staleFrames)(\.ts)?$/.test(source))
            return null;
          const resolved = await this.resolve(source, importer, { ...options, skipSelf: true });
          return aliases.get(resolved?.id) ?? null;
        },
      },
    ],
  });
  return join(out, "dist");
}

/** Serve `dist` on an ephemeral loopback port; `/api` goes to the temporary backend. */
export async function serveViewer(root, dist, api) {
  const require = createRequire(join(root, "web/package.json"));
  const { preview } = await import(require.resolve("vite"));
  const server = await preview({
    root: join(root, "web"),
    configFile: false,
    logLevel: "warn",
    build: { outDir: dist },
    preview: {
      host: "127.0.0.1",
      port: 0,
      proxy: { "/api": { target: api, changeOrigin: true } },
    },
  });
  const { port } = server.httpServer.address();
  return {
    url: `http://127.0.0.1:${port}`,
    async stop() {
      server.httpServer.closeAllConnections?.();
      await server.close();
    },
  };
}
