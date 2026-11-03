// Public listener smoke against an existing disposable local Kubernetes fixture.
// node docker/kubernetes-public-smoke.mjs HTTPS_ORIGIN /tmp/state.json
import assert from "node:assert/strict";
import http from "node:http";
import { readFile } from "node:fs/promises";
import { connectMcp } from "../spikes/e2e/protocol.mjs";

const [origin, statePath] = process.argv.slice(2);
const publicUrl = new URL(origin);
assert.equal(publicUrl.protocol, "https:");
assert.equal(publicUrl.origin, origin);
const fixture = JSON.parse(await readFile(statePath, "utf8"));
assert.match(fixture.projectId, /^p_[a-f0-9]{32}$/);
assert.match(fixture.slug, /^kubernetes-smoke-\d+$/);
const projectPath = `/api/v1/projects/${fixture.projectId}`;

// Native HTTP preserves the supplied Host; fetch may normalize it to loopback.
function request(path, { method = "GET", headers = {}, body } = {}) {
  return new Promise((resolve, reject) => {
    const req = http.request({ hostname: "127.0.0.1", port: 3858, path, method,
      headers: { host: publicUrl.host, origin, ...headers }, timeout: 35_000 }, (res) => {
      const chunks = [];
      res.on("data", (chunk) => chunks.push(chunk));
      res.on("error", reject);
      res.on("end", () => resolve({ status: res.statusCode, headers: res.headers,
        bytes: Buffer.concat(chunks) }));
    });
    req.on("error", reject);
    req.on("timeout", () => req.destroy(new Error(`Timeout: ${method} ${path}`)));
    req.end(body);
  });
}
async function json(path, options) {
  const response = await request(path, options);
  assert.equal(response.status, 200, `${path}: ${response.bytes}`);
  return JSON.parse(response.bytes);
}
async function resource(url) {
  const address = new URL(url);
  assert.equal(address.origin, origin, "generated links use public HTTPS origin");
  const response = await request(address.pathname + address.search);
  assert.equal(response.status, 200, url);
  assert.ok(response.bytes.length > 0, url);
  return response;
}

const before = await json(`${projectPath}/snapshot`);
assert.equal(before.revision, fixture.revision);
assert.equal((await request("/")).status, 200);
assert.deepEqual(await json("/api/v1/viewer"), { readOnly: true });
assert.ok((await json("/api/v1/projects")).projects.some((p) => p.projectId === fixture.projectId));
assert.equal((await request("/api/v1/projects", { method: "HEAD" })).status, 200);
const font = await request("/api/v1/fonts/font_bundled");
assert.equal(font.status, 200);
assert.equal(font.headers["content-type"], "font/ttf");
assert.ok(font.bytes.length > 1000);
for (const headers of [
  { host: "127.0.0.1:3858" }, { host: "evil.example" },
  { origin: "https://evil.example" }, { origin: "http://127.0.0.1:3857" },
]) assert.equal((await request("/api/v1/projects", { headers })).status, 403);

const spoofed = { "x-forwarded-for": "127.0.0.1", "x-forwarded-host": "localhost",
  "x-forwarded-proto": "http", forwarded: "for=127.0.0.1;host=localhost;proto=http",
  "content-type": "application/json" };
let blocked = 0;
for (const path of ["/mcp", "/MCP", "/%6dcp", "/mcp/tools"]) {
  assert.equal((await request(path, { headers: spoofed })).status, 403);
  blocked++;
}
for (const path of ["/mcp", "/api/v1/projects", `${projectPath}/fonts`,
  `${projectPath}/exports/`, `${projectPath}/%65xports`, "/unknown"]) {
  for (const method of ["POST", "PUT", "PATCH", "DELETE", "OPTIONS"]) {
    const response = await request(path, { method, headers: spoofed,
      body: JSON.stringify({ slug: "forbidden", name: "Forbidden" }) });
    assert.equal(response.status, 403, `${method} ${path}`);
    assert.equal(JSON.parse(response.bytes).error.code, "PUBLIC_READ_ONLY");
    blocked++;
  }
}

let job = await json(`${projectPath}/scene?frameId=frame&revision=${fixture.revision}&width=320&height=200`);
const deadline = Date.now() + 90_000;
while (job.status === "pending" && Date.now() < deadline) {
  await new Promise((resolve) => setTimeout(resolve, 200));
  job = await json(`/api/v1/jobs/${job.jobId}`);
}
assert.equal(job.status, "ready", JSON.stringify(job));
assert.equal(job.scene.provenance.revision, fixture.revision);
assert.ok(job.scene.fonts.length > 0);
for (const item of [...job.scene.fonts, ...job.scene.assets]) await resource(item.url);
const png = await resource(job.imageUrl);
assert.equal(job.image.uri, job.imageUrl);
assert.equal(png.headers["content-type"], "image/png");
assert.equal(png.bytes.subarray(0, 8).toString("hex"), "89504e470d0a1a0a");
assert.equal(png.bytes.readUInt32BE(16), 320);
assert.equal(png.bytes.readUInt32BE(20), 200);

const exports = await json(`${projectPath}/exports`, {
  method: "POST", headers: { "content-type": "application/json" },
  body: JSON.stringify({ nodeIds: ["frame"], settings: [{ format: "svg" }, { format: "png" }],
    revision: fixture.revision, waitMs: 10_000 }),
});
const item = exports.exports[0];
assert.equal(item.status, "ready", JSON.stringify(exports));
assert.equal(item.files.length, 2);
for (const file of item.files) {
  const downloaded = await resource(file.url);
  assert.match(downloaded.headers["content-disposition"], /^attachment/);
  if (file.format === "svg") assert.match(downloaded.bytes.toString(), /data:font\/ttf;base64,/);
}
assert.deepEqual(await json(`${projectPath}/snapshot`), before, "public reads/exports preserve authored snapshot");
const local = "http://127.0.0.1:3857";
const internal = await (await fetch(`${local}/api/v1/jobs/${job.jobId}`,
  { signal: AbortSignal.timeout(10_000) })).json();
assert.equal(new URL(internal.imageUrl).origin, local);
assert.equal(new URL(internal.scene.fonts[0].url).origin, local);
const mcp = await connectMcp(local);
try {
  const outline = await mcp.call("get_outline", { projectId: fixture.projectId });
  assert.match(JSON.stringify(outline), /frame/);
  console.log(JSON.stringify({ ok: true, ...fixture, origin, blocked,
    pngBytes: png.bytes.length, exports: item.files.map((f) => f.format),
    protocol: mcp.protocol, imageUrl: job.imageUrl }));
} finally { await mcp.close(); }
