import assert from "node:assert/strict";
import { execFileSync } from "node:child_process";
import { mkdtemp, rm, writeFile } from "node:fs/promises";
import os from "node:os";
import path from "node:path";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { root } from "../../worker/runtime.mjs";
import { fixture } from "./fixture.mjs";
import { prepare } from "./prepare.mjs";

const directory = await mkdtemp(
  path.join(os.tmpdir(), "opencanvas-wrap-proposal-"),
);
const oldBinary = process.env.OPENCANVAS_LAYOUT_BIN;
const evidence = {
  scope:
    "Temporary engine copy: only wrap DTO + Taffy mapping, NOT application acceptance",
  productionValidationImplemented: false,
  startedAt: new Date().toISOString(),
  cases: [],
};
let worker;
let native = false;
const epsilon = 1 / 64;
const near = (a, b) => assert.ok(Math.abs(a - b) <= epsilon, `${a} != ${b}`);
const cards = (scene) => scene.nodes.filter((n) => /^card-\d$/.test(n.id));
const rows = (scene) => new Set(cards(scene).map((n) => n.bounds.y)).size;

try {
  const prepared = await prepare(directory);
  native = prepared.native;
  process.env.OPENCANVAS_LAYOUT_BIN = prepared.binary;
  if (native) {
    evidence.scope =
      "Disposable native engine copy with production validation; historical prototype evidence is preserved separately";
    evidence.productionValidationImplemented = true;
  }
  worker = await PipelineWorker.start();
  evidence.browserVersion = worker.runtime.browserVersion;
  evidence.nodeVersion = process.version;
  async function compute(label, input) {
    const authored = structuredClone(input);
    const result = await worker.compute(input);
    assert.deepEqual(input, authored);
    assert.ok(result.metrics.layoutPasses <= 6);
    evidence.cases.push({ label, viewport: input.viewport, ...result });
    return result;
  }
  for (const [width, count, height] of [
    [1040, 1, 80],
    [640, 2, 156],
    [390, 3, 212],
    [920, 1, 80],
    [919, 2, 156],
    [624, 2, 156],
    [623, 3, 212],
  ]) {
    const { scene } = await compute(`geometry-${width}`, fixture(width));
    assert.equal(rows(scene), count);
    const row = scene.nodes.find((n) => n.id === "cards");
    near(row.bounds.height, height);
    near(row.bounds.width, width - 48);
    for (const card of cards(scene)) {
      assert.ok(
        card.bounds.x >= 24 &&
          card.bounds.x + card.bounds.width <= width - 24 + epsilon,
      );
    }
  }
  for (const [width, count] of [
    [1040, 1],
    [640, 2],
    [390, 3],
  ]) {
    const input = fixture(width, { text: true });
    const result = await compute(`browser-text-${width}`, input);
    const { scene } = result;
    assert.equal(rows(scene), count);
    const row = scene.nodes.find((n) => n.id === "cards");
    const items = cards(scene);
    const groups = Map.groupBy(items, (n) => n.bounds.y);
    const lineHeights = [...groups.values()].map((line) =>
      Math.max(...line.map((n) => n.bounds.height)),
    );
    near(
      row.bounds.height,
      lineHeights.reduce((a, b) => a + b, 0) + (count - 1) * 16,
    );
    for (const card of items) {
      const text = scene.nodes.find((n) => n.id === `${card.id}-text`);
      near(card.bounds.height, text.bounds.height + 32);
      near(text.bounds.height, text.lines.length * 24);
      assert.ok(text.lines.length >= 2);
      assert.ok(
        card.bounds.x >= 0 &&
          card.bounds.x + card.bounds.width <= width + epsilon,
      );
      assert.ok(card.bounds.y + card.bounds.height <= 844);
      for (const line of text.lines)
        assert.ok(line.width <= text.bounds.width + epsilon);
    }
    const sorted = [...groups.entries()].sort(([a], [b]) => a - b);
    for (let i = 1; i < sorted.length; i++) {
      near(sorted[i][0], sorted[i - 1][0] + lineHeights[i - 1] + 16);
    }
    const png = await worker.render(scene);
    await writeFile(
      new URL(`./${native ? "native-" : ""}text-${width}.png`, import.meta.url),
      Buffer.from(png.base64, "base64"),
    );
    const repeat = await worker.compute(input);
    assert.equal(repeat.metrics.batchRPCs, 0);
    assert.deepEqual(repeat.scene, scene);
  }
  const noWrap = await compute(
    "no-wrap-overflow-control",
    fixture(390, { wrap: false }),
  );
  assert.equal(rows(noWrap.scene), 1);
  assert.ok(cards(noWrap.scene).some((n) => n.bounds.x + n.bounds.width > 390));
  if (native) {
    await assert.rejects(
      worker.compute(fixture(390, { min: null })),
      /minWidth/,
    );
    evidence.cases.push({
      label: "basis-zero-no-minimum-control",
      rejected: "LAYOUT_INVALID: explicit positive minWidth required",
    });
  } else {
    const noMinimum = await compute(
      "basis-zero-no-minimum-control",
      fixture(390, { min: null }),
    );
    assert.equal(rows(noMinimum.scene), 1);
  }
  const overflow = await compute(
    "too-large-minimum-control",
    fixture(390, { min: 344 }),
  );
  assert.ok(
    cards(overflow.scene).some((n) => n.bounds.x + n.bounds.width > 390 - 24),
  );
  for (const width of [1040, 390]) {
    const input = fixture(width);
    delete input.nodes[1].props.layout.wrap;
    const reference = JSON.parse(
      execFileSync(path.join(root, "target/debug/layout-step"), {
        input: JSON.stringify({ input, cache: {} }) + "\n",
        encoding: "utf8",
        timeout: 5000,
      }),
    );
    assert.equal(reference.status, "ready");
    const result = await compute(`omitted-default-compat-${width}`, input);
    assert.deepEqual(result.scene.nodes, reference.scene.nodes);
  }
  evidence.success = true;
} finally {
  await worker?.close();
  if (oldBinary === undefined) delete process.env.OPENCANVAS_LAYOUT_BIN;
  else process.env.OPENCANVAS_LAYOUT_BIN = oldBinary;
  await rm(directory, { recursive: true, force: true });
  evidence.temporaryDirectoryRemoved = true;
  evidence.completedAt = new Date().toISOString();
  await writeFile(
    new URL(
      native ? "./native-evidence.json" : "./evidence.json",
      import.meta.url,
    ),
    JSON.stringify(evidence, null, 2) + "\n",
  );
}
console.log(
  JSON.stringify(
    {
      success: evidence.success,
      cases: evidence.cases.length,
      temporaryDirectoryRemoved: evidence.temporaryDirectoryRemoved,
      geometry: evidence.cases
        .filter((c) => c.scene)
        .map(({ label, scene, metrics }) => ({
          label,
          rows: rows(scene),
          rowHeight: scene.nodes.find((n) => n.id === "cards").bounds.height,
          passes: metrics.layoutPasses,
          batchRPCs: metrics.batchRPCs,
        })),
    },
    null,
    2,
  ),
);
