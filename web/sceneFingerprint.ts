import { createHash } from "node:crypto";
import { existsSync, readdirSync, readFileSync } from "node:fs";
import { fileURLToPath } from "node:url";

/** Hash actual dirty sources and deployed WASM bytes, rather than the last Git commit. */
export function sceneFingerprint() {
  const root = fileURLToPath(new URL("../", import.meta.url));
  const hash = createHash("sha256");
  const visit = (path: string) => {
    if (!existsSync(root + path)) {
      hash.update(`missing:${path}\n`);
      return;
    }
    const entries = readdirSync(root + path, { withFileTypes: true });
    for (const entry of entries.sort((a, b) => a.name.localeCompare(b.name))) {
      const name = `${path}/${entry.name}`;
      if (entry.isDirectory()) {
        if (
          !["node_modules", "target", "dist", "build", ".git"].includes(
            entry.name,
          )
        )
          visit(name);
      } else if (entry.isFile())
        hash.update(name).update(readFileSync(root + name));
    }
  };
  for (const path of [
    "web/src",
    "renderer",
    "crates/scene-wasm/src",
    "crates/scene-input/src",
    "crates/layout/src",
    "crates/text/src",
    "web/public/scene-wasm",
  ])
    visit(path);
  for (const path of ["Cargo.lock", "Cargo.toml", "web/package-lock.json"])
    if (existsSync(root + path))
      hash.update(path).update(readFileSync(root + path));
  return hash.digest("hex");
}
