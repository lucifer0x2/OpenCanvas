import { fixtureData } from "./fixture-data.mjs";
import { createServer } from "node:http";
import { createHash } from "node:crypto";
import { readFile } from "node:fs/promises";

export async function startFixture() {
  const font = await readFile(
    new URL("../../renderer/fonts/NotoSans.ttf", import.meta.url),
  );
  const hash = createHash("sha256").update(font).digest("hex");
  const streams = new Set();
  const jobs = new Map();
  const requests = [];
  let revision = 7;
  let mode = "ready";
  let deleted = false;
  const { nodes, scene } = fixtureData(hash, () => ({ mode, deleted }));
  const server = createServer((req, res) => {
    requests.push({ method: req.method, url: req.url });
    const url = new URL(req.url, "http://fixture");
    const json = (data) => {
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify(data));
    };
    if (url.pathname === "/api/v1/font") {
      res.writeHead(200, { "content-type": "font/ttf" });
      res.end(font);
      return;
    }
    if (url.pathname.endsWith("/events")) {
      res.writeHead(200, {
        "content-type": "text/event-stream",
        "cache-control": "no-cache",
      });
      res.write(": connected\n\n");
      streams.add(res);
      req.on("close", () => streams.delete(res));
      return;
    }
    if (url.pathname === "/api/v1/projects")
      return json({
        projects: [
          {
            projectId: "fixture",
            slug: "fixture",
            name: "Viewer test fixture",
            revision,
          },
        ],
      });
    if (url.pathname.endsWith("/snapshot"))
      return json({
        schemaVersion: 1,
        projectId: "fixture",
        slug: "fixture",
        name: "Viewer test fixture",
        revision,
        nodes: nodes(),
        tokens: [],
        styles: [],
        assets: [],
        fonts: [],
        truncated: false,
      });
    if (url.pathname.endsWith("/scene")) {
      const rev = Number(url.searchParams.get("revision"));
      const computed = scene(
        rev,
        Number(url.searchParams.get("width")),
        Number(url.searchParams.get("height")),
      );
      const job = {
        jobId: `job-${rev}`,
        projectId: "fixture",
        revision: rev,
        frameId: "frame",
        status:
          mode === "failed"
            ? "failed"
            : mode === "pending"
              ? "pending"
              : "ready",
        sceneKey: computed.provenance.sourceSceneKey,
        scene: computed,
        error:
          mode === "failed"
            ? { code: "RENDER_FAILED", message: "Fixture worker failure." }
            : undefined,
      };
      jobs.set(job.jobId, job);
      return json(job);
    }
    if (url.pathname.startsWith("/api/v1/jobs/"))
      return json(jobs.get(url.pathname.split("/").at(-1)));
    res.writeHead(404, { "content-type": "application/json" });
    res.end(
      JSON.stringify({
        error: { code: "NOT_FOUND", message: "Fixture resource missing" },
      }),
    );
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return {
    baseURL: `http://127.0.0.1:${server.address().port}`,
    requests,
    update(rev, nextMode = "ready", remove = false) {
      revision = rev;
      mode = nextMode;
      deleted = remove;
      for (const stream of streams)
        stream.write(
          `id: fixture:${rev}\nevent: commit\ndata: ${JSON.stringify({ projectId: "fixture", revision: rev, changedIds: ["title"] })}\n\n`,
        );
    },
    reset() {
      for (const stream of streams)
        stream.write(
          `event: reset_required\ndata: ${JSON.stringify({ projectId: "fixture", currentRevision: revision })}\n\n`,
        );
    },
    complete(rev) {
      const job = jobs.get(`job-${rev}`);
      if (job) job.status = "ready";
    },
    disconnect() {
      for (const stream of streams) stream.end();
      streams.clear();
    },
    close() {
      for (const stream of streams) stream.end();
      server.closeAllConnections();
      return new Promise((resolve) => server.close(resolve));
    },
  };
}
