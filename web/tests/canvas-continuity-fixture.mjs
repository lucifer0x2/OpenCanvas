import { createServer } from "node:http";
import { fixtureData } from "./fixture-data.mjs";

/** Controlled GET/SSE fixture: no storage, worker, fonts or user projects. */
export async function startContinuityFixture() {
  let revision = 7,
    mode = "ready",
    frameWidth = 800;
  const jobs = new Map(),
    streams = new Set(),
    requests = [];
  const data = fixtureData("none", () => ({}));
  const server = createServer((req, res) => {
    requests.push({ method: req.method, url: req.url });
    const url = new URL(req.url, "http://fixture");
    const projectId = url.pathname.split("/")[4] || "fixture";
    const json = (value) => {
      res.writeHead(200, { "content-type": "application/json" });
      res.end(JSON.stringify(value));
    };
    if (url.pathname.endsWith("/events")) {
      res.writeHead(200, { "content-type": "text/event-stream" });
      res.write(": connected\n\n");
      streams.add(res);
      req.on("close", () => streams.delete(res));
      return;
    }
    if (url.pathname === "/api/v1/projects")
      return json({
        projects: ["fixture", "second"].map((id) => ({
          projectId: id,
          slug: id,
          name: id,
          revision,
        })),
      });
    if (url.pathname.endsWith("/snapshot")) {
      const nodes = data.nodes();
      nodes.find((node) => node.id === "frame").props.width = frameWidth;
      nodes.push({
        id: "other",
        parentId: "page",
        sortKey: 2,
        kind: "frame",
        props: { name: "Other frame", width: 500, height: 400 },
      });
      return json({
        schemaVersion: 1,
        projectId,
        revision,
        nodes,
        truncated: false,
      });
    }
    if (url.pathname.endsWith("/scene")) {
      const rev = Number(url.searchParams.get("revision"));
      const width = Number(url.searchParams.get("width"));
      const height = Number(url.searchParams.get("height"));
      const frameId = url.searchParams.get("frameId");
      const scene = data.scene(rev, width, height);
      Object.assign(scene.provenance, { projectId, frameId });
      scene.fonts = [];
      scene.nodes = scene.nodes.filter((node) => node.kind !== "text");
      const job = {
        jobId: `${projectId}-${frameId}-${rev}-${width}-${height}`,
        projectId,
        revision: rev,
        frameId,
        status: mode === "mismatch" ? "ready" : mode,
        sceneKey: scene.provenance.sourceSceneKey,
        scene,
        error: { code: "RENDER_FAILED", message: "Controlled render failure." },
      };
      if (mode === "mismatch") job.revision--;
      jobs.set(job.jobId, job);
      return json(job);
    }
    if (url.pathname.startsWith("/api/v1/jobs/"))
      return json(jobs.get(url.pathname.split("/").at(-1)));
    res.writeHead(404).end();
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return {
    baseURL: `http://127.0.0.1:${server.address().port}`,
    requests,
    setMode(next) {
      mode = next;
    },
    update(rev, next = "ready", width = frameWidth) {
      revision = rev;
      mode = next;
      frameWidth = width;
      for (const stream of streams)
        stream.write(
          `id: fixture:${rev}\nevent: commit\ndata: ${JSON.stringify({ projectId: "fixture", revision: rev })}\n\n`,
        );
    },
    complete(rev) {
      for (const job of jobs.values())
        if (job.revision === rev) job.status = "ready";
    },
    async close() {
      for (const stream of streams) stream.end();
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
    },
  };
}
