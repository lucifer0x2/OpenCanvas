import { build } from "esbuild";
import { fileURLToPath } from "node:url";

/** Bundle TS exactly as the app does, without requiring Node to resolve Vite's extensionless imports. */
export async function sceneModule(name) {
  const result = await build({
    entryPoints: [
      fileURLToPath(new URL(`../src/scene/${name}.ts`, import.meta.url)),
    ],
    bundle: true,
    write: false,
    format: "esm",
    platform: "node",
    target: "es2022",
  });
  return import(
    `data:text/javascript;base64,${Buffer.from(result.outputFiles[0].text).toString("base64")}`
  );
}
