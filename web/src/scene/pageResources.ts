import { loadResources } from "../../../renderer/index";
import type { Resources, Scene } from "../../../renderer/types";
import type { FontBytesProvider } from "../../../renderer/resource-bytes";

/** One load per immutable manifest across this board, with at most four resource loads at once. */
export function pageResourceLoader() {
  const known = new Set<string>();
  const resources: Resources = {
    fonts: new Map(),
    images: new Map(),
    assetHashes: new Map(),
    readiness: [],
    diagnostics: [],
    glyphCoverage: new Map(),
  };
  const load = async (
    scene: Scene,
    signal: AbortSignal,
    fontBytes?: FontBytesProvider,
  ) => {
    const loads: (() => Promise<Resources>)[] = [];
    for (const [kind, manifests] of [
      ["font", scene.fonts],
      ["asset", scene.assets],
    ] as const)
      for (const manifest of manifests) {
        const key = JSON.stringify([kind, manifest]);
        if (known.has(key)) continue;
        known.add(key);
        loads.push(() =>
          loadResources(
            kind === "font" ? [manifest as Scene["fonts"][number]] : [],
            kind === "asset" ? [manifest as Scene["assets"][number]] : [],
            { strict: false, fontBytes },
          ),
        );
      }
    for (let offset = 0; offset < loads.length; offset += 4) {
      signal.throwIfAborted();
      const loaded = await Promise.all(
        loads.slice(offset, offset + 4).map((load) => load()),
      );
      signal.throwIfAborted();
      for (const value of loaded) {
        for (const key of [
          "fonts",
          "images",
          "assetHashes",
          "glyphCoverage",
        ] as const)
          for (const [id, entry] of value[key] ?? [])
            (resources[key] as Map<string, unknown>).set(id, entry);
        resources.readiness.push(...value.readiness);
        resources.diagnostics.push(...value.diagnostics);
      }
    }
    const readiness = resources.readiness.filter((item) =>
      (item.kind === "font" ? scene.fonts : scene.assets).some(
        (manifest) => manifest.id === item.id,
      ),
    );
    return { ...resources, readiness, diagnostics: [...resources.diagnostics] };
  };
  // Server jobs may finish together: a shared manifest must finish before any consumer publishes it.
  let queue = Promise.resolve();
  return (...args: Parameters<typeof load>) => {
    const result = queue.then(() => load(...args));
    queue = result.then(
      () => undefined,
      () => undefined,
    );
    return result;
  };
}
