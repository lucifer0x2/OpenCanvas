import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdir, writeFile } from "node:fs/promises";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { root } from "../../worker/runtime.mjs";
import { responsiveFixture as fixture } from "./responsive-fixture.mjs";
import { assertResponsive, cards, near } from "./responsive-assert.mjs";

let worker;
const output = new URL("./output/", import.meta.url);
before(async () => {
  worker = await PipelineWorker.start();
  await mkdir(output, { recursive: true });
});
after(async () => {
  await worker?.close();
});

test("native wrap geometry, exact thresholds and fixed row height align start", async () => {
  for (const [width, count, height] of [
    [1040, 1, 80],
    [640, 2, 156],
    [390, 3, 212],
    [920, 1, 80],
    [919, 2, 156],
    [624, 2, 156],
    [623, 3, 212],
  ]) {
    const input = fixture(width),
      original = structuredClone(input);
    const { scene, metrics } = await worker.compute(input);
    assertResponsive(scene, count, height);
    assert.deepEqual(input, original);
    assert.equal(metrics.batchRPCs, 0);
  }
  const input = fixture(390);
  input.nodes[1].props.height = 500;
  const { scene } = await worker.compute(input);
  assert.deepEqual(
    cards(scene).map((n) => n.bounds.y),
    [24, 80, 156],
  );
});

test("native wrap real Vietnamese/unbroken text, bounded batching, cache and PNG", async () => {
  for (const [width, count, height] of [
    [1040, 1, 128],
    [640, 2, 224],
    [390, 3, 320],
  ]) {
    const input = fixture(width, { text: true }),
      original = structuredClone(input);
    const result = await worker.compute(input);
    assertResponsive(result.scene, count, height, true);
    assert.ok(result.metrics.layoutPasses <= 6);
    assert.equal(result.metrics.batchRPCs, 1);
    const repeat = await worker.compute(input);
    assert.equal(repeat.metrics.batchRPCs, 0);
    assert.deepEqual(repeat.scene, result.scene);
    assert.deepEqual(input, original);
    const png = await worker.render(result.scene);
    await writeFile(
      new URL(`wrap-${width}.png`, output),
      Buffer.from(png.base64, "base64"),
    );
    await writeFile(
      new URL(`wrap-${width}.json`, output),
      JSON.stringify(result, null, 2),
    );
  }
});

test("no-wrap omitted/false compatibility and explicit oversized minimum overflow", async () => {
  for (const width of [1040, 390]) {
    const input = fixture(width, { wrap: false });
    const explicit = await worker.compute(input);
    delete input.nodes[1].props.layout.wrap;
    const omitted = await worker.compute(input);
    assert.deepEqual(explicit.scene, omitted.scene);
    if (width === 390)
      assert.ok(
        cards(omitted.scene).some((n) => n.bounds.x + n.bounds.width > 390),
      );
  }
  const oversized = await worker.compute(fixture(390, { min: 344 }));
  assert.ok(
    cards(oversized.scene).some((n) => n.bounds.x + n.bounds.width > 366),
  );
  await assert.rejects(worker.compute(fixture(390, { min: null })), /minWidth/);
});

test("nested finite fill chain, fixed/hug children and clamped wrap height", async () => {
  const input = fixture(390);
  input.nodes.splice(1, 0, {
    id: "outer",
    parentId: "frame",
    kind: "group",
    props: { width: "fill", height: "hug", layout: { mode: "column" } },
  });
  input.nodes[2].parentId = "outer";
  input.nodes[3].props = { width: 60, minWidth: 80, maxWidth: 80, height: 40 };
  input.nodes[4].props = { width: "hug", minWidth: 120, height: 60 };
  input.nodes[5].props = { width: 280, height: 80 };
  input.nodes[2].props.layout.padding = 4;
  input.nodes[2].props.maxHeight = 100;
  const { scene } = await worker.compute(input);
  const row = scene.nodes.find((n) => n.id === "cards");
  near(row.bounds.width, 342);
  near(row.bounds.height, 100);
  assert.deepEqual(
    cards(scene).map((n) => n.bounds.width),
    [80, 120, 280],
  );
  assert.deepEqual(
    cards(scene).map((n) => n.bounds.y),
    [28, 28, 104],
  );
  assert.ok(cards(scene)[2].bounds.y + 80 > row.bounds.y + row.bounds.height);
});

test("layout CLI rejects malformed wrap before publishing measurements", () => {
  for (const value of [null, "true", 1, { tokenId: "w" }]) {
    const input = fixture(390, { text: true });
    input.nodes[1].props.layout.wrap = value;
    const result = JSON.parse(
      execFileSync(`${root}/target/debug/layout-step`, {
        input: JSON.stringify({ input, cache: {} }) + "\n",
        encoding: "utf8",
      }),
    );
    assert.equal(result.status, "failed");
    assert.equal(result.code, "INVALID_JSON");
    assert.match(result.message, /boolean/);
  }
});
