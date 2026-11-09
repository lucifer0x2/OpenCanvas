import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";
import { fixtureData } from "./fixture-data.mjs";

/** Complete temporary authored snapshot; every font and scene request is recorded. */
export async function startSceneFixture() {
  const bytes = await readFile(
    new URL("../../renderer/fonts/NotoSans.ttf", import.meta.url),
  );
  const hash = createHash("sha256").update(bytes).digest("hex");
  const data = fixtureData(hash, () => ({}));
  const requests = [],
    streams = new Set(),
    held = [];
  let revision = 7,
    hold = false,
    corruptFont = false;
  const json = (res, value) => {
    res.writeHead(200, { "content-type": "application/json" });
    res.end(JSON.stringify(value));
  };
  const server = createServer((req, res) => {
    requests.push(req.url);
    const url = new URL(req.url, "http://fixture");
    if (url.pathname === "/api/v1/viewer/auth")
      return json(res, { authenticated: true, required: false });
    if (url.pathname === "/api/v1/projects")
      return json(res, {
        projects: [
          {
            projectId: "fixture",
            slug: "fixture",
            name: "WASM fixture",
            revision,
          },
        ],
      });
    if (url.pathname.endsWith("/events")) {
      res.writeHead(200, { "content-type": "text/event-stream" });
      res.write(": connected\n\n");
      streams.add(res);
      req.on("close", () => streams.delete(res));
      return;
    }
    if (url.pathname.endsWith("/snapshot")) {
      const respond = () => {
        const nodes = structuredClone(data.nodes());
        nodes.find((node) => node.id === "title").props.text =
          `Accepted snapshot revision ${revision}`;
        json(res, {
          schemaVersion: 1,
          projectId: "fixture",
          slug: "fixture",
          name: "WASM fixture",
          revision,
          nodes,
          tokens: [],
          styles: [],
          assets: [],
          fonts: [
            {
              id: "font_bundled",
              family: "OpenCanvas Noto Sans",
              weight: 400,
              style: "normal",
              assetId: null,
              bundledId: "font_bundled",
              contentHash: hash,
              license: "OFL",
              status: "ready",
            },
            {
              id: "unused_missing",
              family: "Unused unavailable face",
              weight: 400,
              style: "normal",
              assetId: "missing",
              bundledId: null,
              contentHash: "0".repeat(64),
              license: null,
              status: "missing",
            },
          ],
          retention: { oldestCommitRevision: 1, idempotencyPolicy: "retained" },
          truncated: false,
        });
      };
      if (hold) held.push(respond);
      else respond();
      return;
    }
    if (url.pathname.endsWith("/fonts/font_bundled/download")) {
      res.writeHead(200, { "content-type": "font/ttf" });
      res.end(corruptFont ? Buffer.from("corrupt font") : bytes);
      return;
    }
    if (url.pathname.endsWith("/scene")) {
      const rev = Number(url.searchParams.get("revision"));
      const width = Number(url.searchParams.get("width")),
        height = Number(url.searchParams.get("height"));
      const scene = data.scene(rev, width, height);
      scene.fonts = [];
      scene.nodes = scene.nodes.filter((node) => node.kind !== "text");
      return json(res, {
        projectId: "fixture",
        frameId: "frame",
        revision: rev,
        status: "ready",
        jobId: `server-${rev}`,
        sceneKey: scene.provenance.sourceSceneKey,
        scene,
      });
    }
    res.writeHead(404).end();
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return {
    baseURL: `http://127.0.0.1:${server.address().port}`,
    requests,
    holdSnapshots(value) {
      hold = value;
      if (!hold) for (const respond of held.splice(0)) respond();
    },
    corruptFont(value) {
      corruptFont = value;
    },
    update(next) {
      revision = next;
      for (const stream of streams)
        stream.write(
          `id: fixture:${next}\nevent: commit\ndata: ${JSON.stringify({ projectId: "fixture", revision: next })}\n\n`,
        );
    },
    async close() {
      for (const stream of streams) stream.end();
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
    },
  };
}
