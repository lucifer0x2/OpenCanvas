// Temporary backend lifecycle for one scenario: a fresh OS temp data root,
// random loopback port, and guaranteed cleanup. Never touches ~/.opencanvas or
// port 3847 (startServer is always given port 0).
import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";
import { startServer } from "../e2e/server.mjs";
import { connectMcp } from "../e2e/protocol.mjs";

/**
 * Runs `body(env)` with a temp data root. `env.start()` / `env.restart()`
 * (re)launch the production binary on a random port; `env.connect()` opens an
 * MCP session that is closed automatically. Everything is torn down in finally.
 */
export async function withEnvironment(body) {
  const root = await mkdtemp(join(tmpdir(), "opencanvas-agent-cost-"));
  const sessions = [];
  let server;
  const env = {
    root,
    get base() {
      return server.base;
    },
    async start() {
      server = await startServer(root, 0);
      return server;
    },
    async restart() {
      await closeSessions(sessions);
      await server?.stop();
      server = undefined;
      return env.start();
    },
    async connect() {
      const mcp = await connectMcp(server.base);
      sessions.push(mcp);
      return mcp;
    },
  };
  const cleanup = { serverStopped: false, rootRemoved: false };
  try {
    await env.start();
    const result = await body(env);
    return { result, cleanup };
  } finally {
    await closeSessions(sessions);
    if (server) await server.stop();
    cleanup.serverStopped = true;
    await rm(root, { recursive: true, force: true });
    cleanup.rootRemoved = true;
  }
}

async function closeSessions(sessions) {
  await Promise.all(sessions.splice(0).map((mcp) => mcp.close()));
}

/**
 * Seeds with an unmetered session, then restarts the backend so the measured
 * session starts cold: no prerender jobs queued by the seeding commits, no
 * in-memory render cache, the same state an agent sees opening an existing
 * project.
 */
export async function seedThenRestart(env, seed) {
  const setup = await env.connect();
  const seeded = await seed(setup);
  await env.restart();
  return seeded;
}
