// Smoke test for a running OpenCanvas server, e.g. the Docker image:
//   node docker/smoke.mjs http://127.0.0.1:38470
// Plain Node (>= 18, global fetch), no dependencies. It creates one project named smoke-<time>
// in the server's data root. Prints a JSON summary; exits 1 when any step fails.
const base = (process.argv[2] ?? "http://127.0.0.1:3847").replace(/\/$/, "");
const LEGACY = "2025-11-25";
const MODERN = "2026-07-28";
const ACCEPT = "application/json, text/event-stream";
// Tools that must stay published. New tools need no change here: the check is "contains these,
// no duplicates", never a count.
const REQUIRED_TOOLS = [
  "list_projects", "create_project", "get_outline", "find_nodes", "get_subtree",
  "get_design_context", "get_changes", "list_tokens", "list_styles", "list_assets",
  "list_fonts", "import_asset", "apply_batch", "render_frame", "inspect_layout",
];
const steps = [];
let legacyTools;
let sequence = 0;

async function step(name, run) {
  const started = Date.now();
  try {
    const detail = await run();
    steps.push({ step: name, ok: true, ms: Date.now() - started, ...detail });
  } catch (error) {
    steps.push({ step: name, ok: false, error: String(error?.message ?? error) });
    throw error;
  }
}

function check(condition, message) {
  if (!condition) throw new Error(message);
}

/** Checks a tools/list result and returns its tool names plus its compact JSON size. */
function checkTools(list) {
  const names = list.tools.map((tool) => tool.name);
  check(new Set(names).size === names.length, `duplicate tools: ${names}`);
  const missing = REQUIRED_TOOLS.filter((name) => !names.includes(name));
  check(missing.length === 0, `missing tools: ${missing}`);
  return { tools: names.length, toolsBytes: Buffer.byteLength(JSON.stringify(list.tools)), names };
}

/** One JSON-RPC message from a JSON or SSE body (SSE opens with an empty priming event). */
function decode(text) {
  const data = text
    .split("\n")
    .filter((line) => line.startsWith("data: "))
    .map((line) => line.slice(6))
    .find((line) => line.trim() !== "");
  return JSON.parse(data ?? text);
}

async function post(body, headers) {
  const response = await fetch(`${base}/mcp`, {
    method: "POST",
    headers: { accept: ACCEPT, "content-type": "application/json", ...headers },
    body: JSON.stringify(body),
  });
  const text = await response.text();
  return { response, message: text ? decode(text) : null };
}

async function rpc(method, params, headers) {
  const { response, message } = await post(
    { jsonrpc: "2.0", id: ++sequence, method, params },
    headers,
  );
  check(response.ok, `${method}: HTTP ${response.status}`);
  check(!message?.error, `${method}: ${JSON.stringify(message?.error)}`);
  return { response, result: message.result };
}

let session;
const legacy = (method, params = {}) =>
  rpc(method, params, { "mcp-session-id": session, "mcp-protocol-version": LEGACY });
const modern = (method, params = {}) =>
  rpc(
    method,
    {
      ...params,
      _meta: {
        "io.modelcontextprotocol/protocolVersion": MODERN,
        "io.modelcontextprotocol/clientCapabilities": {},
        "io.modelcontextprotocol/clientInfo": { name: "docker-smoke", version: "1" },
      },
    },
    { "mcp-protocol-version": MODERN, "mcp-method": method },
  );

async function tool(name, args) {
  const { result } = await legacy("tools/call", { name, arguments: args });
  const content = result.structuredContent;
  check(!result.isError, `${name}: ${JSON.stringify(content?.error ?? result)}`);
  return content;
}

const operations = [
  { op: "create_node", id: "page", kind: "page", parentId: null, props: { name: "Smoke" } },
  {
    op: "create_node", id: "frame", kind: "frame", parentId: "page",
    props: { name: "Card", width: 320, height: 200, fill: "#ffffff" },
  },
  {
    op: "create_node", id: "text", kind: "text", parentId: "frame",
    props: {
      text: "Docker smoke OpenCanvas", width: 280, height: "hug",
      fontId: "font_bundled", fontSize: 20, lineHeight: 28,
    },
  },
];

async function main() {
  await step("viewer", async () => {
    const response = await fetch(`${base}/`);
    const html = await response.text();
    check(response.ok, `HTTP ${response.status}`);
    check(/<html/i.test(html), "viewer index.html missing");
    return { status: response.status };
  });
  await step("legacy initialize + tools/list", async () => {
    const init = { protocolVersion: LEGACY, capabilities: {}, clientInfo: { name: "docker-smoke", version: "1" } };
    const { response, result } = await rpc("initialize", init);
    session = response.headers.get("mcp-session-id");
    check(session, "no Mcp-Session-Id");
    check(result.protocolVersion === LEGACY, `negotiated ${result.protocolVersion}`);
    const { response: ack } = await post(
      { jsonrpc: "2.0", method: "notifications/initialized" },
      { "mcp-session-id": session, "mcp-protocol-version": LEGACY },
    );
    check(ack.ok, `notifications/initialized: HTTP ${ack.status}`);
    const { result: list } = await legacy("tools/list");
    const { names, ...summary } = checkTools(list);
    legacyTools = names;
    return { protocolVersion: result.protocolVersion, ...summary };
  });
  await step("2026 discover + tools/list", async () => {
    const { result: discover } = await modern("server/discover");
    check(discover.supportedVersions?.includes(MODERN), "2026-07-28 not supported");
    const { response, result } = await modern("tools/list");
    check(!response.headers.get("mcp-session-id"), "2026 request opened a session");
    const { names, ...summary } = checkTools(result);
    check(names.join() === legacyTools?.join(), "2026 and legacy tools/list differ");
    check(Number.isInteger(result.ttlMs) && result.cacheScope, "ttlMs/cacheScope missing");
    return { ...summary, ttlMs: result.ttlMs, cacheScope: result.cacheScope };
  });
  let projectId;
  await step("create_project", async () => {
    const slug = `smoke-${Date.now()}`;
    projectId = (await tool("create_project", { slug, name: "Docker smoke" })).projectId;
    check(projectId, "no projectId");
    return { slug, projectId };
  });
  await step("apply_batch page/frame/text", async () => {
    const body = { projectId, expectedRevision: 0, idempotencyKey: "docker-smoke", operations };
    const committed = await tool("apply_batch", body);
    check(committed.committedRevision === 1, JSON.stringify(committed));
    return { revision: committed.committedRevision };
  });
  let job;
  await step("render_frame + waitMs", async () => {
    const viewport = { width: 320, height: 200 };
    job = await tool("render_frame", { projectId, revision: 1, frameId: "frame", viewport });
    for (let polls = 0; job.status === "pending" && polls < 6; polls++)
      job = await tool("render_frame", { projectId, jobId: job.jobId, waitMs: 10_000 });
    check(job.status === "ready", `job ${job.jobId}: ${JSON.stringify(job.error ?? job.status)}`);
    return { jobId: job.jobId, status: job.status, browser: job.provenance?.browserVersion };
  });
  await step("PNG via imageUrl", async () => {
    // imageUrl names the container's own port; reach the same path through the given base.
    const url = new URL(job.imageUrl);
    const response = await fetch(`${base}${url.pathname}`);
    const bytes = new Uint8Array(await response.arrayBuffer());
    const signature = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];
    check(response.ok, `HTTP ${response.status}`);
    check(response.headers.get("content-type") === "image/png", "not image/png");
    check(signature.every((byte, i) => bytes[i] === byte), "bad PNG signature");
    const view = new DataView(bytes.buffer);
    const size = { width: view.getUint32(16), height: view.getUint32(20) };
    check(size.width === 320, `PNG width ${size.width}`);
    return { imageUrl: job.imageUrl, bytes: bytes.length, ...size };
  });
}

main()
  .catch(() => {})
  .finally(() => {
    const ok = steps.length === 7 && steps.every((s) => s.ok);
    console.log(JSON.stringify({ base, ok, steps }, null, 1));
    process.exitCode = ok ? 0 : 1;
  });
