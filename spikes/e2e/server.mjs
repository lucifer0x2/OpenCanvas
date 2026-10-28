import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import { waitFor } from "./protocol.mjs";

export const workspace = fileURLToPath(new URL("../../", import.meta.url));
export async function startServer(dataRoot, port = 0) {
  const child = spawn(
    `${workspace}/target/debug/opencanvas-server`,
    ["--data-root", dataRoot, "--port", String(port), "--workspace", workspace],
    { cwd: workspace, detached: true, stdio: ["ignore", "pipe", "pipe"] },
  );
  let output = "";
  child.stdout.on("data", (data) => {
    output += data;
  });
  child.stderr.on("data", (data) => {
    output += data;
  });
  child.on("error", (error) => {
    output += error.message;
  });
  let base;
  try {
    base = await waitFor(
      async () => {
        assert.equal(child.exitCode, null, output);
        return output.match(/OpenCanvas (http:\/\/127\.0\.0\.1:\d+)/)?.[1];
      },
      "production backend startup",
      10_000,
    );
  } catch (error) {
    child.kill("SIGTERM");
    throw error;
  }
  return {
    base,
    port: Number(new URL(base).port),
    output: () => output,
    async stop() {
      if (child.exitCode !== null || child.signalCode !== null) return;
      const exited = once(child, "exit");
      // Group termination includes the Node worker and Chromium children.
      process.kill(-child.pid, "SIGTERM");
      await exited;
    },
  };
}
