// Byte/latency metering for MCP tool calls made through spikes/e2e/protocol.mjs.
// protocol.mjs calls the global `fetch`, so the meter taps that instead of
// re-implementing the transport: the request body is measured as sent and the
// response stream is copied while protocol.mjs reads it.

/** Wraps global fetch for POST <base>/mcp; restore() must run in `finally`. */
export function installMeter() {
  const original = globalThis.fetch;
  const exchanges = [];
  globalThis.fetch = async (input, init = {}) => {
    if (init.method !== "POST" || !String(input).endsWith("/mcp"))
      return original(input, init);
    const body = String(init.body ?? "");
    const request = JSON.parse(body);
    const exchange = {
      method: request.method,
      id: request.id,
      tool: request.params?.name,
      requestBytes: Buffer.byteLength(body),
      chunks: [],
      contentType: "",
    };
    exchanges.push(exchange);
    const response = await original(input, init);
    exchange.contentType = response.headers.get("content-type") ?? "";
    if (!response.body) return response;
    const tap = new TransformStream({
      transform(chunk, controller) {
        exchange.chunks.push(chunk);
        controller.enqueue(chunk);
      },
    });
    return new Response(response.body.pipeThrough(tap), {
      status: response.status,
      statusText: response.statusText,
      headers: response.headers,
    });
  };
  return {
    exchanges,
    restore() {
      globalThis.fetch = original;
    },
  };
}

/**
 * Size of the JSON-RPC response message for this request, excluding SSE
 * framing and priming events: what an MCP client hands to the model.
 */
export function responseBytes(exchange) {
  const text = Buffer.concat(
    exchange.chunks.map((c) => Buffer.from(c)),
  ).toString("utf8");
  if (!exchange.contentType.includes("text/event-stream"))
    return Buffer.byteLength(text);
  for (const event of text.split(/\r?\n\r?\n/)) {
    const data = event
      .split(/\r?\n/)
      .filter((line) => line.startsWith("data:"))
      .map((line) => line.slice(5).trim())
      .join("\n");
    if (!data) continue;
    try {
      if (JSON.parse(data).id === exchange.id) return Buffer.byteLength(data);
    } catch {
      // A partial trailing event is never the matched response.
    }
  }
  throw new Error(
    `No response captured for ${exchange.method} #${exchange.id}`,
  );
}

const round = (ms) => Math.round(ms * 100) / 100;

/**
 * Per-scenario call log. `client(mcp, name)` returns an object whose `call`
 * mirrors protocol.mjs `call` and records {tool, requestBytes, responseBytes,
 * ms}. Calls must be sequential: each record takes the tools/call exchange that
 * started after the previous one.
 */
export function recorder(meter) {
  const calls = [];
  let phase = "main";
  let started;
  let ended;
  function client(mcp, name) {
    return {
      async call(tool, args, expectedError) {
        const from = meter.exchanges.length;
        started ??= performance.now();
        const t0 = performance.now();
        const value = await mcp.call(tool, args, expectedError);
        const ms = performance.now() - t0;
        ended = performance.now();
        const exchange = meter.exchanges
          .slice(from)
          .find((e) => e.method === "tools/call");
        if (!exchange || exchange.tool !== tool)
          throw new Error(`Meter lost the ${tool} exchange`);
        calls.push({
          tool,
          phase,
          ...(name ? { client: name } : {}),
          requestBytes: exchange.requestBytes,
          responseBytes: responseBytes(exchange),
          structuredBytes: Buffer.byteLength(JSON.stringify(value)),
          ms: round(ms),
          ...(expectedError ? { error: expectedError } : {}),
        });
        exchange.chunks = [];
        return value;
      },
    };
  }
  return {
    calls,
    client,
    phase(name) {
      phase = name;
    },
    wallMs: () => (started === undefined ? 0 : round(ended - started)),
  };
}

/** Totals for a list of call records. */
export function totals(calls) {
  const requestBytes = calls.reduce((sum, c) => sum + c.requestBytes, 0);
  const responseBytes = calls.reduce((sum, c) => sum + c.responseBytes, 0);
  const structuredBytes = calls.reduce((sum, c) => sum + c.structuredBytes, 0);
  return {
    calls: calls.length,
    requestBytes,
    responseBytes,
    bytes: requestBytes + responseBytes,
    structuredBytes,
    callMs: round(calls.reduce((sum, c) => sum + c.ms, 0)),
  };
}

/** Totals grouped by phase, in first-seen order. */
export function byPhase(calls) {
  const groups = new Map();
  for (const call of calls) {
    if (!groups.has(call.phase)) groups.set(call.phase, []);
    groups.get(call.phase).push(call);
  }
  return Object.fromEntries(
    [...groups].map(([name, list]) => [name, totals(list)]),
  );
}

/** Latency distribution of call records (nearest-rank percentiles). */
export function latency(calls) {
  const values = calls.map((c) => c.ms).sort((a, b) => a - b);
  const rank = (p) =>
    values[
      Math.min(values.length - 1, Math.ceil((p / 100) * values.length) - 1)
    ];
  return {
    samples: values.length,
    min: values[0],
    p50: rank(50),
    p95: rank(95),
    max: values.at(-1),
    mean: round(values.reduce((a, b) => a + b, 0) / values.length),
  };
}
