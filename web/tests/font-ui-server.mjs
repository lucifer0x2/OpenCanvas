import { spawn } from "node:child_process";
import { once } from "node:events";
import { fileURLToPath } from "node:url";
import { waitFor } from "../../spikes/e2e/protocol.mjs";

/** Serve source through the real dev proxy; never rewrite web/dist or touch port3847. */
export async function startFontUi(backend) {
  const child = spawn(
    process.execPath,
    ["node_modules/vite/bin/vite.js", "--host", "127.0.0.1", "--port", "0"],
    {
      cwd: fileURLToPath(new URL("../", import.meta.url)),
      env: { ...process.env, OPENCANVAS_API_URL: backend },
      stdio: ["ignore", "pipe", "pipe"],
    },
  );
  let output = "";
  child.stdout.on("data", (value) => {
    output += value;
  });
  child.stderr.on("data", (value) => {
    output += value;
  });
  const base = await waitFor(
    () => output.match(/http:\/\/127\.0\.0\.1:\d+/)?.[0],
    "font UI Vite startup",
  );
  return {
    base,
    async stop() {
      const exit = once(child, "exit");
      child.kill("SIGTERM");
      await exit;
    },
  };
}
