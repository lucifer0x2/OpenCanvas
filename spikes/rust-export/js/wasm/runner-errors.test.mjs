// Integration failure probes after building/generating Task09 inputs. Restore every input in finally.
import test from "node:test";
import assert from "node:assert/strict";
import { renameSync, readFileSync, writeFileSync } from "node:fs";
import { spawnSync } from "node:child_process";
import { fileURLToPath } from "node:url";

const spike = fileURLToPath(new URL("../../", import.meta.url));
function fails(pattern, env = {}, variant = "nosimd-release") {
  const r = spawnSync(process.execPath, [spike + "js/wasm-bench.mjs", variant], {
    cwd: fileURLToPath(new URL("../../../../", import.meta.url)),
    encoding: "utf8", timeout: 15_000, env: { ...process.env, ...env },
  });
  assert.equal(r.error, undefined, "runner must exit, not hang waiting for HTTP/browser cleanup");
  assert.equal(r.status, 1);
  assert.match(r.stderr, pattern);
}

test("missing WASM is nonzero before browser launch", () => {
  const f = spike + "out/wasm/rust_spike-nosimd-release.wasm";
  renameSync(f, f + ".probe");
  try { fails(/ENOENT/); }
  finally { renameSync(f + ".probe", f); }
});

test("missing native RGBA is nonzero, never silently skips parity", () => {
  const f = spike + "out/wasm/native/spike-s1.rgba";
  renameSync(f, f + ".probe");
  try { fails(/ENOENT/); }
  finally { renameSync(f + ".probe", f); }
});

test("missing native case rejects stale/incomplete coverage", () => {
  const f = spike + "out/wasm/native.json", original = readFileSync(f);
  try {
    const json = JSON.parse(original);
    json.cases.pop();
    writeFileSync(f, JSON.stringify(json));
    fails(/Missing\/extra benchmark cases/);
  } finally { writeFileSync(f, original); }
});

test("launch failure after listen closes the HTTP server", () => {
  fails(/Executable doesn't exist/, { PLAYWRIGHT_BROWSERS_PATH: spike + "out/wasm/absent-browser" });
});
