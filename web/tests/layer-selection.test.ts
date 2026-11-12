import assert from "node:assert/strict";
import test from "node:test";
import { layerSelectionPath } from "../src/layerSelection.ts";
import type { DesignNode } from "../src/types.ts";

const node = (id: string, parentId: string | null, kind = "group", props = {}): DesignNode =>
  ({ id, parentId, kind, props, sortKey: 0 });
const nodes = [
  node("page", null, "page"), node("frame", "page", "frame"),
  node("left", "frame"), node("right", "frame"),
  node("leaf", "left", "rectangle"),
  node("component", null, "component"), node("text", "component", "text"),
  node("instance", "right", "instance", { componentId: "component" }),
];

test("selection identifies only its ancestry and omits the page", () => {
  assert.deepEqual(layerSelectionPath(nodes, "leaf"), {
    id: "frame", child: { id: "left", child: { id: "leaf", child: undefined } },
  });
  assert.deepEqual(layerSelectionPath(nodes, "frame"), { id: "frame", child: undefined });
  assert.equal(layerSelectionPath(nodes, ""), undefined);
  assert.equal(layerSelectionPath(nodes, "removed"), undefined);
});

test("virtual instance descendants keep their frame ancestry without selecting the instance row", () => {
  assert.deepEqual(layerSelectionPath(nodes, "instance/text"), {
    id: "frame", child: { id: "right", child: {
      id: "instance", child: { id: "instance/text", child: undefined },
    } },
  });
  assert.equal(layerSelectionPath(nodes, "instance/leaf"), undefined);
});

test("malformed cyclic parents terminate instead of hanging selection", () => {
  const path = layerSelectionPath([node("a", "b"), node("b", "a")], "a");
  assert.deepEqual(path, { id: "b", child: { id: "a", child: undefined } });
});
