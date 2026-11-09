import { serverFrameJob } from "./all-frames-scene.mjs";
import { createServer } from "node:http";
import { readFile } from "node:fs/promises";
import { createHash } from "node:crypto";

export const PAGE_SIZES = {
  three: [
    [390, 844],
    [390, 844],
    [390, 844],
  ],
  zero: [],
  one: [[800, 600]],
  four: Array.from({ length: 4 }, () => [800, 600]),
  nine: Array.from({ length: 9 }, () => [800, 600]),
  mixed: [
    [390, 1200],
    [1440, 900],
    [600, 3000],
    [2400, 300],
  ],
};
export async function startAllFramesFixture() {
  const bytes = await readFile(
    new URL("../../renderer/fonts/NotoSans.ttf", import.meta.url),
  );
  const hash = createHash("sha256").update(bytes).digest("hex");
  const requests = [],
    streams = new Set(),
    active = new Map();
  let revision = 7,
    failure = "",
    maxActive = 0,
    overlap = false;
  const font = {
    id: "font_bundled",
    family: "OpenCanvas Noto Sans",
    weight: 400,
    style: "normal",
    assetId: null,
    bundledId: "font_bundled",
    contentHash: hash,
    license: "OFL",
    status: "ready",
  };
  const nodes = () =>
    Object.entries(PAGE_SIZES).flatMap(([page, sizes], sortKey) => [
      {
        id: page,
        kind: "page",
        parentId: null,
        sortKey,
        props: { name: page },
      },
      ...sizes.flatMap(([width, height], index) => {
        const id = `${page}-${index}`;
        return [
          {
            id,
            kind: "frame",
            parentId: page,
            sortKey: index,
            props: {
              name: `${page} / Frame ${index + 1}`,
              width,
              height,
              fill: "#ffffff",
            },
          },
          {
            id: `${id}-shape`,
            kind: "rectangle",
            parentId: id,
            sortKey: 0,
            props: {
              name: `Accent ${index + 1}`,
              x: 24,
              y: 32,
              width: Math.min(220, width - 48),
              height: 160,
              fill: ["#dae8f0", "#dfeadd", "#eee0ce"][index % 3],
            },
          },
          {
            id: `${id}-title`,
            kind: "text",
            parentId: id,
            sortKey: 1,
            props: {
              name: `Title ${index + 1}`,
              x: 24,
              y: 220,
              width: width - 48,
              height: 48,
              text: `Frame ${index + 1}`,
              fontId: "font_bundled",
              fontSize: 24,
              lineHeight: 32,
              fill: "#26364a",
            },
          },
        ];
      }),
    ]);
  const job = (...args) => serverFrameJob(hash, ...args);
  const json = (res, value, status = 200) => {
    res.writeHead(status, { "content-type": "application/json" });
    res.end(JSON.stringify(value));
  };
  const server = createServer((req, res) => {
    requests.push(req.url);
    const url = new URL(req.url, "http://fixture"),
      path = url.pathname;
    if (path.endsWith("/viewer/auth"))
      return json(res, { authenticated: true, required: false });
    if (path === "/api/v1/projects")
      return json(res, {
        projects: [
          {
            projectId: "all-fixture",
            slug: "fixture",
            name: "All frames fixture",
            revision,
          },
        ],
      });
    if (path.endsWith("/events")) {
      res.writeHead(200, { "content-type": "text/event-stream" });
      res.write(": connected\n\n");
      streams.add(res);
      req.on("close", () => streams.delete(res));
      return;
    }
    if (path.endsWith("/snapshot"))
      return json(res, {
        schemaVersion: 1,
        projectId: "all-fixture",
        slug: "fixture",
        name: "All frames fixture",
        revision,
        nodes: nodes(),
        fonts: [font],
        assets: [],
        tokens: [],
        styles: [],
        notes: [],
        retention: { oldestCommitRevision: 1, idempotencyPolicy: "retained" },
        truncated: false,
      });
    if (path.endsWith("/fonts/font_bundled/download")) {
      res.writeHead(200, { "content-type": "font/ttf" });
      return res.end(bytes);
    }
    if (path.endsWith("/scene"))
      return json(
        res,
        job(
          url.searchParams.get("frameId"),
          Number(url.searchParams.get("width")),
          Number(url.searchParams.get("height")),
          Number(url.searchParams.get("revision")),
        ),
      );
    if (path.endsWith("/scenes")) {
      if (active.size) overlap = true;
      const pageId = url.searchParams.get("pageId"),
        offset = Number(url.searchParams.get("offset"));
      const sizes = PAGE_SIZES[pageId];
      const jobs = sizes.slice(offset, offset + 4).map(([width, height], i) => {
        const ready = job(
          `${pageId}-${offset + i}`,
          Number(url.searchParams.get("width")) || width,
          Number(url.searchParams.get("height")) || height,
          Number(url.searchParams.get("revision")),
        );
        active.set(ready.jobId, ready);
        return { ...ready, status: "pending", scene: undefined };
      });
      maxActive = Math.max(maxActive, active.size);
      return json(res, {
        projectId: "all-fixture",
        revision,
        pageId,
        jobs,
        total: sizes.length,
        nextOffset:
          offset + jobs.length < sizes.length ? offset + jobs.length : null,
      });
    }
    if (path.startsWith("/api/v1/jobs/")) {
      const key = decodeURIComponent(path.split("/").at(-1)),
        ready = active.get(key);
      active.delete(key);
      return json(
        res,
        ready.frameId === failure
          ? {
              ...ready,
              status: "failed",
              scene: undefined,
              error: { code: "TEST_FAILURE", message: "Fixture render failed" },
            }
          : ready,
      );
    }
    res.writeHead(404).end();
  });
  await new Promise((resolve) => server.listen(0, "127.0.0.1", resolve));
  return {
    baseURL: `http://127.0.0.1:${server.address().port}`,
    requests,
    metrics: () => ({ maxActive, overlap }),
    fail: (id) => {
      failure = id;
    },
    update() {
      revision++;
      for (const stream of streams)
        stream.write(
          `id: all-fixture:${revision}\nevent: commit\ndata: ${JSON.stringify({ projectId: "all-fixture", revision })}\n\n`,
        );
    },
    async close() {
      for (const stream of streams) stream.end();
      server.closeAllConnections();
      await new Promise((resolve) => server.close(resolve));
    },
  };
}
