// Read-only acceptance: authored snapshots, actual MCP, image render and browser WASM bytes.
import assert from "node:assert/strict";
import { createHash } from "node:crypto";
import { mkdir, writeFile } from "node:fs/promises";
import { connectMcp, json, waitFor } from "../../spikes/e2e/protocol.mjs";

const [base = "http://127.0.0.1:3867", output = ".runtime/native/rehearsal/http", reference] = process.argv.slice(2);
await mkdir(output, { recursive: true, mode: 0o700 });
const digest = (bytes) => createHash("sha256").update(bytes).digest("hex");
const list = await json(base, "/api/v1/projects");
assert.ok(list.projects.length > 0);
assert.equal(list.nextCursor, null);
const snapshots = [];
for (const project of list.projects) {
  const route = `/api/v1/projects/${encodeURIComponent(project.projectId)}/snapshot`;
  const response = await fetch(new URL(route, base));
  assert.equal(response.status, 200);
  const raw = await response.text();
  const snapshot = JSON.parse(raw);
  assert.equal(snapshot.revision, project.revision);
  await writeFile(`${output}/${project.projectId}.json`, raw, { mode: 0o600 });
  if (reference) {
    const original = await (await fetch(new URL(route, reference))).text();
    assert.equal(raw.replaceAll(base, reference), original, "authored snapshot matches source");
  }
  snapshots.push({ project, snapshot, sha256: digest(raw) });
}
const mcp = await connectMcp(base);
let png;
try {
  const listed = await mcp.call("list_projects", {});
  assert.equal(listed.projects.length, list.projects.length);
  const selected = snapshots.find(({ snapshot }) => snapshot.nodes.some((node) => node.kind === "frame"));
  assert.ok(selected, "existing design has a renderable frame");
  const frame = selected.snapshot.nodes.find((node) => node.kind === "frame");
  const outline = await mcp.call("get_outline", { projectId: selected.project.projectId });
  assert.ok(JSON.stringify(outline).includes(frame.id));
  const query = new URLSearchParams({ frameId: frame.id, revision: String(selected.project.revision), width: "320", height: "200" });
  let job = await json(base, `/api/v1/projects/${selected.project.projectId}/scene?${query}`);
  job = await waitFor(async () => {
    if (job.status === "pending") job = await json(base, `/api/v1/jobs/${job.jobId}`);
    return job.status === "pending" ? null : job;
  }, "native Node/Chromium PNG", 90_000);
  assert.equal(job.status, "ready", JSON.stringify(job));
  const image = await fetch(job.imageUrl);
  assert.equal(image.status, 200);
  assert.match(image.headers.get("content-type"), /^image\/png/);
  png = Buffer.from(await image.arrayBuffer());
  assert.equal(png.subarray(0, 8).toString("hex"), "89504e470d0a1a0a");
  assert.equal(png.readUInt32BE(16), 320);
  assert.equal(png.readUInt32BE(20), 200);
  await writeFile(`${output}/frame.png`, png, { mode: 0o600 });
} finally {
  await mcp.close();
}
for (const file of ["opencanvas_scene_wasm.js", "opencanvas_scene_wasm_bg.wasm"]) {
  const response = await fetch(`${base}/scene-wasm/${file}`);
  assert.equal(response.status, 200);
  const bytes = Buffer.from(await response.arrayBuffer());
  if (file.endsWith(".wasm")) {
    assert.match(response.headers.get("content-type"), /^application\/wasm/);
    assert.equal(bytes.subarray(0, 4).toString("hex"), "0061736d");
  } else assert.match(response.headers.get("content-type"), /javascript/);
}
const result = { ok: true, projectCount: list.projects.length, revisions: list.projects.map((p) => p.revision).sort((a, b) => a - b),
  protocol: mcp.protocol, snapshots: snapshots.map(({ project, sha256 }) => ({ projectId: project.projectId, revision: project.revision, sha256 })),
  pngBytes: png.length, pngSha256: digest(png), wasmBytesVerified: true };
await writeFile(`${output}/result.json`, JSON.stringify(result, null, 2), { mode: 0o600 });
console.log(JSON.stringify({ ok: true, projectCount: result.projectCount, revisions: result.revisions, protocol: result.protocol,
  pngBytes: result.pngBytes, wasmBytesVerified: true }));
