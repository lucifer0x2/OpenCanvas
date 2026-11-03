// Local-cluster smoke. create writes a labelled disposable fixture; check only
// reads its saved identity after a Pod restart. State contains no credentials.
// node docker/kubernetes-smoke.mjs create|check http://127.0.0.1:3857 /tmp/state.json
import assert from "node:assert/strict";
import { readFile, writeFile } from "node:fs/promises";
import { connectMcp } from "../spikes/e2e/protocol.mjs";

const [mode, address = "http://127.0.0.1:3857", statePath] = process.argv.slice(2);
assert.ok(["create", "check"].includes(mode) && statePath, "create|check BASE STATE required");
const base = new URL(address).origin;
assert.equal(base, "http://127.0.0.1:3857", "Use the isolated local Kubernetes port");
const request = (path) => fetch(new URL(path, base), { signal: AbortSignal.timeout(10_000) });
const json = async (path) => {
  const response = await request(path);
  assert.equal(response.status, 200, path);
  return response.json();
};
const response = await request("/");
assert.equal(response.status, 200);
assert.match(await response.text(), /<html/i);
const mcp = await connectMcp(base);
try {
  let fixture;
  if (mode === "create") {
    const slug = `kubernetes-smoke-${Date.now()}`;
    const project = await mcp.call("create_project", { slug, name: "Disposable Kubernetes smoke" });
    fixture = { projectId: project.projectId, slug, revision: 1 };
    // Save before mutation so a failed smoke still identifies only its own data.
    await writeFile(statePath, JSON.stringify(fixture, null, 2), { mode: 0o600, flag: "wx" });
    const committed = await mcp.call("apply_batch", {
      projectId: fixture.projectId, expectedRevision: 0, idempotencyKey: slug,
      operations: [
        { op: "create_node", id: "page", kind: "page", parentId: null, props: { name: "Smoke" } },
        { op: "create_node", id: "frame", kind: "frame", parentId: "page",
          props: { width: 320, height: 200, fill: "#ffffff" } },
        { op: "create_node", id: "text", kind: "text", parentId: "frame",
          props: { text: "Kubernetes PostgreSQL smoke", width: 280, height: "hug",
            fontId: "font_bundled", fontSize: 20, lineHeight: 28 } },
      ],
    });
    assert.equal(committed.committedRevision, fixture.revision);
  } else {
    fixture = JSON.parse(await readFile(statePath, "utf8"));
    assert.match(fixture.slug, /^kubernetes-smoke-\d+$/);
  }
  const projects = await mcp.call("list_projects", {});
  assert.ok(projects.projects.some((p) => p.projectId === fixture.projectId && p.slug === fixture.slug));
  const snapshot = await json(`/api/v1/projects/${fixture.projectId}/snapshot`);
  assert.equal(snapshot.revision, fixture.revision);
  assert.equal(snapshot.nodes.find((node) => node.id === "text")?.props.text,
    "Kubernetes PostgreSQL smoke");
  // Both HTTP and MCP must read the committed tree, including after restarts.
  const outline = await mcp.call("get_outline", { projectId: fixture.projectId });
  assert.match(JSON.stringify(outline), /frame/);
  let job = await mcp.call("render_frame", {
    projectId: fixture.projectId, revision: fixture.revision, frameId: "frame",
    viewport: { width: 320, height: 200 }, includeScene: true,
  });
  const deadline = Date.now() + 90_000;
  while (job.status === "pending" && Date.now() < deadline) {
    job = await mcp.call("render_frame", {
      projectId: fixture.projectId, jobId: job.jobId, waitMs: 10_000, includeScene: true,
    });
  }
  assert.equal(job.status, "ready", JSON.stringify(job.error ?? job.status));
  assert.equal(job.scene.provenance.revision, fixture.revision);
  assert.equal(new URL(job.imageUrl).origin, base, "generated image URL must work unchanged");
  const png = await request(job.imageUrl);
  assert.equal(png.status, 200);
  assert.equal(png.headers.get("content-type"), "image/png");
  const bytes = Buffer.from(await png.arrayBuffer());
  assert.equal(bytes.subarray(0, 8).toString("hex"), "89504e470d0a1a0a");
  assert.equal(bytes.readUInt32BE(16), 320);
  assert.equal(bytes.readUInt32BE(20), 200);
  const font = await request("/api/v1/fonts/font_bundled");
  assert.equal(font.status, 200);
  assert.equal(font.headers.get("content-type"), "font/ttf");
  assert.ok((await font.arrayBuffer()).byteLength > 1000);
  assert.ok(job.readiness.every((resource) => resource.status === "ready"));
  console.log(JSON.stringify({ ok: true, mode, ...fixture, imageUrl: job.imageUrl,
    pngBytes: bytes.length, sceneKey: job.sceneKey, protocol: mcp.protocol }));
} finally {
  await mcp.close();
}
