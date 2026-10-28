import assert from "node:assert/strict";

export async function json(base, route) {
  const response = await fetch(new URL(route, base));
  const value = await response.json();
  assert.equal(response.status, 200, JSON.stringify(value));
  return value;
}

export async function waitFor(check, label, timeout = 45_000) {
  const until = Date.now() + timeout;
  while (Date.now() < until) {
    const value = await check();
    if (value) return value;
    await new Promise((resolve) => setTimeout(resolve, 100));
  }
  throw new Error(`Timed out: ${label}`);
}

// Exercise the actual Streamable HTTP transport, including negotiated headers.
export async function connectMcp(base) {
  let session;
  let protocol;
  let id = 0;
  const calls = [];
  async function rpc(method, params = {}, notification = false) {
    const body = { jsonrpc: "2.0", method, params };
    if (!notification) body.id = ++id;
    const headers = {
      "content-type": "application/json",
      accept: "application/json, text/event-stream",
    };
    if (session) headers["mcp-session-id"] = session;
    if (protocol) headers["mcp-protocol-version"] = protocol;
    const response = await fetch(`${base}/mcp`, {
      method: "POST",
      headers,
      body: JSON.stringify(body),
      signal: AbortSignal.timeout(60_000),
    });
    session ??= response.headers.get("mcp-session-id");
    if (!response.ok)
      throw new Error(
        `${method}: HTTP ${response.status} ${await response.text()}`,
      );
    calls.push(method);
    if (notification) {
      await response.arrayBuffer();
      return;
    }
    let value;
    if (response.headers.get("content-type")?.includes("text/event-stream")) {
      const reader = response.body.getReader();
      const decoder = new TextDecoder();
      let buffer = "";
      try {
        while (!value) {
          const chunk = await reader.read();
          if (chunk.done) break;
          buffer += decoder.decode(chunk.value, { stream: true });
          const events = buffer.split(/\r?\n\r?\n/);
          buffer = events.pop();
          for (const event of events) {
            const data = event
              .split(/\r?\n/)
              .filter((line) => line.startsWith("data:"))
              .map((line) => line.slice(5).trim())
              .join("\n");
            if (data) {
              const parsed = JSON.parse(data);
              if (parsed.id === body.id) value = parsed;
            }
          }
        }
      } finally {
        await reader.cancel();
      }
    } else value = await response.json();
    assert.ok(value, `${method}: no response`);
    assert.equal(value.error, undefined, JSON.stringify(value.error));
    return value.result;
  }
  const info = await rpc("initialize", {
    protocolVersion: "2025-11-25",
    capabilities: {},
    clientInfo: { name: "OpenCanvas live acceptance", version: "1.0.0" },
  });
  protocol = info.protocolVersion;
  assert.ok(session, "transport must create a session");
  await rpc("notifications/initialized", {}, true);
  const tools = (await rpc("tools/list")).tools;
  const call = async (name, args, expectedError) => {
    const result = await rpc("tools/call", { name, arguments: args });
    const value = result.structuredContent;
    assert.ok(value, `No structured result for ${name}`);
    if (expectedError) {
      assert.equal(result.isError, true, JSON.stringify(value));
      assert.equal(value.error.code, expectedError, JSON.stringify(value));
    } else assert.ok(!result.isError, `${name}: ${JSON.stringify(value)}`);
    return value;
  };
  return {
    call,
    tools,
    calls,
    protocol,
    server: info.serverInfo,
    close: () =>
      fetch(`${base}/mcp`, {
        method: "DELETE",
        headers: {
          "mcp-session-id": session,
          "mcp-protocol-version": protocol,
        },
        signal: AbortSignal.timeout(5_000),
      }).catch(() => {}),
  };
}
