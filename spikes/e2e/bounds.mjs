import assert from "node:assert/strict";

// Logical scene pixels, before any ancestor clipping. Root is never a descendant.
export const EPSILON = 1e-6;
const dimensions = ["x", "y", "width", "height"];

export function descendants(nodes, frameId) {
  assert.equal(new Set(nodes.map((n) => n.id)).size, nodes.length);
  assert.ok(
    nodes.some((n) => n.id === frameId),
    "missing frame root",
  );
  return nodes
    .filter((n) => n.id !== frameId)
    .map(({ id, bounds }) => {
      assert.ok(
        dimensions.every((key) => Number.isFinite(bounds?.[key])),
        id,
      );
      return { id, bounds };
    });
}

export function horizontalOverflow(nodes, viewport) {
  return nodes
    .flatMap(({ id, bounds: b }) => {
      const leftPx = Math.max(0, -b.x);
      const rightPx = Math.max(0, b.x + b.width - viewport.width);
      return leftPx > EPSILON || rightPx > EPSILON
        ? [{ nodeId: id, leftPx, rightPx, totalPx: leftPx + rightPx }]
        : [];
    })
    .sort((a, b) => a.nodeId.localeCompare(b.nodeId));
}

export function compareBounds(before, after) {
  const first = new Map(before.map((n) => [n.id, n.bounds]));
  const second = new Map(after.map((n) => [n.id, n.bounds]));
  assert.deepEqual(
    [...first.keys()].sort(),
    [...second.keys()].sort(),
    "reflow requires identical complete descendant scope",
  );
  assert.ok(first.size > 0, "empty scope cannot prove reflow");
  const changes = [...first].flatMap(([nodeId, b]) => {
    const a = second.get(nodeId);
    const changedDimensions = dimensions.filter(
      (key) => Math.abs(b[key] - a[key]) > EPSILON,
    );
    return changedDimensions.length
      ? [{ nodeId, changedDimensions, before: b, after: a }]
      : [];
  });
  return {
    reflow: changes.length > 0,
    comparedDescendants: first.size,
    changedNodeIds: changes.map((n) => n.nodeId),
    changes,
  };
}
