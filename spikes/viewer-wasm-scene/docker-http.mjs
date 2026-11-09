import assert from "node:assert/strict";
import { request as httpRequest } from "node:http";
import { request as httpsRequest } from "node:https";
import { brotliDecompressSync, gunzipSync } from "node:zlib";

// Node HTTP keeps wire bytes intact; fetch transparently decompresses its body.
export function raw(base, route, encoding, headersOnly = false) {
  return new Promise((resolve, reject) => {
    const url = new URL(route, base);
    const request = (url.protocol === "https:" ? httpsRequest : httpRequest)(
      url,
      { headers: { "accept-encoding": encoding }, timeout: 30_000 },
      (response) => {
        const result = {
          status: response.statusCode,
          headers: response.headers,
        };
        if (headersOnly) {
          resolve({ ...result, bytes: Buffer.alloc(0) });
          response.destroy();
          return;
        }
        const chunks = [];
        response.on("data", (chunk) => chunks.push(chunk));
        response.on("end", () =>
          resolve({ ...result, bytes: Buffer.concat(chunks) }),
        );
        response.on("error", reject);
      },
    );
    request.on("error", reject);
    request.on("timeout", () =>
      request.destroy(new Error(`HTTP timeout: ${route}`)),
    );
    request.end();
  });
}

export async function compressedAsset(base, route, mime) {
  const identity = await raw(base, route, "identity");
  assert.equal(identity.status, 200, route);
  assert.match(identity.headers["content-type"] ?? "", mime);
  assert.equal(identity.headers["content-encoding"], undefined);
  assert.ok(identity.bytes.length > 0);
  const sizes = { identity: identity.bytes.length };
  for (const [encoding, decode] of [
    ["br", brotliDecompressSync],
    ["gzip", gunzipSync],
  ]) {
    const response = await raw(base, route, encoding);
    assert.equal(response.status, 200, route);
    assert.equal(response.headers["content-encoding"], encoding, route);
    assert.match(response.headers["content-type"] ?? "", mime);
    assert.match(response.headers.vary ?? "", /accept-encoding/i);
    assert.deepEqual(
      decode(response.bytes),
      identity.bytes,
      `${route}: ${encoding} bytes`,
    );
    assert.ok(
      response.bytes.length < identity.bytes.length,
      `${route}: compression gain`,
    );
    sizes[encoding] = response.bytes.length;
  }
  return sizes;
}
