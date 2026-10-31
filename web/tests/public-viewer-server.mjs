// Temporary HTTPS reverse proxy models Funnel while preserving Host/Origin and SSE.
import assert from "node:assert/strict";
import { execFileSync, spawn } from "node:child_process";
import { once } from "node:events";
import { readFile } from "node:fs/promises";
import http from "node:http";
import https from "node:https";
import { fileURLToPath } from "node:url";
import { waitFor } from "../../spikes/e2e/protocol.mjs";

export async function publicViewerServer(root) {
  execFileSync(
    "openssl",
    [
      "req",
      "-x509",
      "-newkey",
      "rsa:2048",
      "-nodes",
      "-days",
      "1",
      "-subj",
      "/CN=127.0.0.1",
      "-keyout",
      `${root}/key.pem`,
      "-out",
      `${root}/cert.pem`,
    ],
    { stdio: "ignore" },
  );
  let upstream;
  const proxy = https.createServer(
    {
      key: await readFile(`${root}/key.pem`),
      cert: await readFile(`${root}/cert.pem`),
    },
    (request, response) => {
      const forwarded = http.request(
        new URL(request.url, upstream),
        {
          method: request.method,
          headers: request.headers,
        },
        (incoming) => {
          response.writeHead(incoming.statusCode, incoming.headers);
          response.flushHeaders();
          incoming.pipe(response);
        },
      );
      forwarded.on("error", () => {
        response.writeHead(502);
        response.end();
      });
      response.on("close", () => forwarded.destroy());
      request.pipe(forwarded);
    },
  );
  proxy.listen(0, "127.0.0.1");
  await once(proxy, "listening");
  const origin = `https://127.0.0.1:${proxy.address().port}`;
  const workspace = fileURLToPath(new URL("../../", import.meta.url));
  const child = spawn(
    `${workspace}/target/debug/opencanvas-server`,
    [
      "--workspace",
      workspace,
      "--data-root",
      `${root}/data`,
      "--port",
      "0",
      "--public-viewer-port",
      "0",
      "--public-viewer-origin",
      origin,
    ],
    { cwd: workspace, detached: true, stdio: ["ignore", "pipe", "pipe"] },
  );
  let output = "";
  child.stdout.on("data", (data) => {
    output += data;
  });
  child.stderr.on("data", (data) => {
    output += data;
  });
  const stop = async () => {
    proxy.closeAllConnections();
    await new Promise((resolve) => proxy.close(resolve));
    if (child.exitCode === null && child.signalCode === null) {
      const exited = once(child, "exit");
      child.kill("SIGINT");
      await exited;
    }
    try {
      process.kill(-child.pid, "SIGTERM");
    } catch {}
  };
  try {
    await waitFor(
      () => {
        assert.equal(child.exitCode, null, output);
        const address = output.match(
          /Public viewer .*bound to (127\.0\.0\.1:\d+)/,
        )?.[1];
        if (address) upstream = `http://${address}`;
        return upstream;
      },
      "temporary public listener",
      10_000,
    );
    return {
      origin,
      local: output.match(/OpenCanvas (http:\/\/127\.0\.0\.1:\d+)/)[1],
      stop,
    };
  } catch (error) {
    await stop();
    throw error;
  }
}
