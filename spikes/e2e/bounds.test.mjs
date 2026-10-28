import assert from "node:assert/strict";
import test from "node:test";
import { descendants, horizontalOverflow, compareBounds } from "./bounds.mjs";

const node = (id, x, width, y = 0, height = 10) => ({
  id,
  bounds: { x, y, width, height },
});
test("reports both negative and right overflow without applying clipping", () => {
  assert.deepEqual(
    horizontalOverflow([node("wide", -12, 140), node("edge", 0, 100)], {
      width: 100,
    }),
    [{ nodeId: "wide", leftPx: 12, rightPx: 28, totalPx: 40 }],
  );
});
test("viewport root changes alone cannot claim descendant reflow", () => {
  const a = descendants([node("root", 0, 100), node("fixed", 70, 30)], "root");
  const b = descendants([node("root", 0, 50), node("fixed", 70, 30)], "root");
  assert.equal(compareBounds(a, b).reflow, false);
  assert.equal(compareBounds(a, b).comparedDescendants, 1);
  assert.equal(horizontalOverflow(b, { width: 50 })[0].rightPx, 50);
});
test("compares all dimensions, tolerates numeric noise and rejects missing scope", () => {
  const a = [
    node("x", 1, 20),
    node("y", 0, 20),
    node("w", 0, 20),
    node("h", 0, 20),
  ];
  const b = [
    node("x", 2, 20),
    node("y", 0, 20, 1),
    node("w", 0, 21),
    node("h", 0, 20, 0, 11),
  ];
  assert.deepEqual(
    compareBounds(a, b).changes.map((c) => c.changedDimensions),
    [["x"], ["y"], ["width"], ["height"]],
  );
  assert.equal(
    compareBounds([node("n", 0, 20)], [node("n", 1e-7, 20)]).reflow,
    false,
  );
  assert.throws(() => compareBounds(a, b.slice(1)), /scope/);
  assert.throws(() => compareBounds([], []), /empty scope/);
});
