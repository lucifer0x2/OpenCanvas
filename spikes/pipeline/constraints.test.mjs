import { after, before, test } from "node:test";
import assert from "node:assert/strict";
import { mkdir, writeFile } from "node:fs/promises";
import { PipelineWorker } from "../../worker/pipeline.mjs";
import { constraintsFixture } from "./constraints-fixture.mjs";

let worker;
before(async () => {
  worker = await PipelineWorker.start();
});
after(async () => {
  await worker?.close();
});

test("nested fixed/fill/hug obey min/max, padding/gap, line height and authored immutability", async () => {
  const input = constraintsFixture();
  const authored = structuredClone(input);
  const { scene, metrics } = await worker.compute(input);
  const byId = Object.fromEntries(scene.nodes.map((n) => [n.id, n]));
  const expected = {
    frame: [0, 0, 360, 320],
    row: [10, 10, 340, 90],
    fixed: [16, 16, 80, 30],
    column: [104, 16, 200, 78],
    text: [108, 20, 150, 70],
    footer: [10, 108, 40, 18],
    overflow: [10, 134, 80, 30],
    "overflow-child": [10, 134, 120, 30],
  };
  for (const [id, [x, y, width, height]] of Object.entries(expected)) {
    assert.deepEqual(byId[id].bounds, { x, y, width, height }, id);
  }
  assert.deepEqual(byId.text.lines, [
    { text: "Một", x: 0, y: 22, width: 29.96875 },
    { text: "Hai", x: 0, y: 50, width: 24.953125 },
  ]);
  assert.deepEqual(input, authored);
  assert.equal(scene.provenance.revision, 3);
  assert.ok(metrics.layoutPasses <= 6);
  await mkdir(new URL("./output/", import.meta.url), { recursive: true });
  await writeFile(
    new URL("./output/constraints.scene.json", import.meta.url),
    JSON.stringify(scene, null, 2),
  );
});

test("fixed overflow keeps authored extent and paints outside only without clip", async () => {
  const sample = async (clip) => {
    const { scene } = await worker.compute(constraintsFixture(clip));
    await worker.render(scene);
    return worker.runtime.page.evaluate(() => {
      const ctx = document.getElementById("scene").getContext("2d");
      return [
        [30, 140],
        [100, 140],
      ].map(([x, y]) => [...ctx.getImageData(x, y, 1, 1).data]);
    });
  };
  assert.deepEqual(await sample(true), [
    [239, 68, 68, 255],
    [255, 255, 255, 255],
  ]);
  assert.deepEqual(await sample(false), [
    [239, 68, 68, 255],
    [239, 68, 68, 255],
  ]);
});
