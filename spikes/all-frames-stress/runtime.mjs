import { spawn, execFileSync } from "node:child_process";
import { mkdtemp, writeFile } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { fileURLToPath } from "node:url";
import { waitFor } from "../e2e/protocol.mjs";

export const ROOT = fileURLToPath(new URL("../../", import.meta.url));
export const OUT = join(ROOT, "artifacts/all-frames-stress");
export const save = (name, value) =>
  writeFile(join(OUT, name), JSON.stringify(value, null, 2));
export const pause = (ms) => new Promise((resolve) => setTimeout(resolve, ms));
export function rss(pid) {
  try {
    return (
      Number(
        execFileSync("ps", ["-o", "rss=", "-p", String(pid)], {
          encoding: "utf8",
        }).trim(),
      ) * 1024
    );
  } catch {
    return null;
  }
}
export async function startBackend(existingRoot) {
  const dataRoot =
    existingRoot ?? (await mkdtemp(join(tmpdir(), "opencanvas-all-stress-")));
  const binary =
    process.env.STRESS_SERVER_BIN ??
    "/Users/luis/Workspace/OpenCanvas/target/debug/opencanvas-server";
  const child = spawn(
    binary,
    ["--data-root", dataRoot, "--port", "0", "--workspace", ROOT],
    {
      cwd: ROOT,
      detached: true,
      stdio: ["ignore", "pipe", "pipe"],
      env: {
        ...process.env,
        DATABASE_PROVIDER: "sqlite",
        RENDER_BACKEND: "rust",
        DATABASE_URL: "",
        PUBLIC_VIEWER_PORT: "",
      },
    },
  );
  let output = "";
  child.stdout.on("data", (bytes) => {
    output += bytes;
  });
  child.stderr.on("data", (bytes) => {
    output += bytes;
  });
  const base = await waitFor(
    () => {
      if (child.exitCode !== null) throw new Error(output);
      return output.match(/OpenCanvas (http:\/\/127\.0\.0\.1:\d+)/)?.[1];
    },
    "temporary native backend",
    15000,
  );
  return {
    base,
    dataRoot,
    pid: child.pid,
    binary,
    async stop() {
      await writeFile(join(OUT, "backend.log"), output);
      if (child.exitCode === null) process.kill(-child.pid, "SIGTERM");
    },
  };
}
