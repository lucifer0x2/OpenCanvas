import { test } from "node:test";
import assert from "node:assert/strict";
import { spawn } from "node:child_process";
import { fixture } from "./fixtures.mjs";
import { root } from "../../worker/runtime.mjs";

test("executable NDJSON worker survives bad request and emits scene plus strict PNG provenance", async () => {
  const input = fixture(200);
  input.assets = [];
  input.nodes = input.nodes.filter((n) => n.kind !== "image");
  const child = spawn(process.execPath, ["worker/cli.mjs"], {
    cwd: root,
    stdio: ["pipe", "pipe", "pipe"],
  });
  let stdout = "",
    stderr = "";
  child.stdout.setEncoding("utf8");
  child.stdout.on("data", (part) => {
    stdout += part;
  });
  child.stderr.setEncoding("utf8");
  child.stderr.on("data", (part) => {
    stderr += part;
  });
  const done = new Promise((resolve, reject) => {
    child.once("error", reject);
    child.once("close", resolve);
  });
  const timeout = setTimeout(() => child.kill(), 20_000);
  child.stdin.end(
    "invalid-json\n" +
      JSON.stringify({ id: "cli-fixture", op: "compute_render", input }) +
      "\n",
  );
  try {
    assert.equal(await done, 0, stderr);
    const [bad, good] = stdout
      .trim()
      .split("\n")
      .map((line) => JSON.parse(line));
    assert.equal(bad.status, "failed");
    assert.equal(good.id, "cli-fixture");
    assert.equal(good.status, "ready");
    assert.equal(good.scene.provenance.revision, 7);
    assert.equal(good.render.mimeType, "image/png");
    assert.equal(
      good.render.provenance.sourceSceneKey,
      good.scene.provenance.sourceSceneKey,
    );
    assert.ok(good.render.base64.startsWith("iVBOR"));
    assert.ok(!JSON.stringify(bad).includes(root));
  } finally {
    clearTimeout(timeout);
    if (child.exitCode === null) child.kill();
  }
});
