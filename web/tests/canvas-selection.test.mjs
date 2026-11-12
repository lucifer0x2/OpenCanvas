import assert from "node:assert/strict";
import test from "node:test";
import { sceneModule } from "./scene-module.mjs";

const { sceneNodes } = await sceneModule("sceneNodes");

test("selection reuses one node index while a board scene survives revisions and zooms", () => {
  const nodes = [{ id: "frame" }, { id: "instance/text" }];
  const scene = { nodes };
  const index = sceneNodes(scene);
  assert.equal(index.get("instance/text"), nodes[1]);
  assert.equal(index.get("missing"), undefined);
  assert.equal(sceneNodes(scene), index);
  const replacement = { nodes: [{ id: "frame" }, { id: "new" }] };
  assert.notEqual(sceneNodes(replacement), index);
  assert.equal(sceneNodes(replacement).has("instance/text"), false);
  assert.equal(sceneNodes(scene).get("instance/text"), nodes[1]);
});
