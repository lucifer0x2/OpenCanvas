import assert from "node:assert/strict";
const epsilon = 1 / 64;
export const near = (a, b) =>
  assert.ok(Math.abs(a - b) <= epsilon, `${a} != ${b}`);
export const cards = (scene) =>
  scene.nodes.filter((n) => /^card-\d$/.test(n.id));

export function assertResponsive(scene, count, height, text = false) {
  const byId = Object.fromEntries(scene.nodes.map((n) => [n.id, n]));
  const groups = [
    ...Map.groupBy(cards(scene), (n) => n.bounds.y).entries(),
  ].sort(([a], [b]) => a - b);
  assert.equal(groups.length, count);
  near(byId.cards.bounds.height, height);
  near(byId.cards.bounds.x, 24);
  near(byId.cards.bounds.y, 24);
  near(byId.cards.bounds.width, scene.provenance.viewport.width - 48);
  for (const n of scene.nodes.filter((n) => n.id !== "frame")) {
    const p = byId[n.parentId];
    assert.ok(
      n.bounds.x >= p.bounds.x - epsilon && n.bounds.y >= p.bounds.y - epsilon,
      n.id,
    );
    assert.ok(
      n.bounds.x + n.bounds.width <= p.bounds.x + p.bounds.width + epsilon,
      n.id,
    );
    assert.ok(
      n.bounds.y + n.bounds.height <= p.bounds.y + p.bounds.height + epsilon,
      n.id,
    );
    if (n.kind === "text") {
      near(n.bounds.height, n.lines.length * 24);
      for (const line of n.lines)
        assert.ok(line.width <= n.bounds.width + epsilon);
      near(p.bounds.height, n.bounds.height + 32);
    }
  }
  for (let i = 0; i < groups.length; i++) {
    const [y, items] = groups[i];
    const expectedWidth =
      (byId.cards.bounds.width - (items.length - 1) * 16) / items.length;
    for (const item of items) near(item.bounds.width, expectedWidth);
    for (let j = 1; j < items.length; j++)
      near(
        items[j].bounds.x,
        items[j - 1].bounds.x + items[j - 1].bounds.width + 16,
      );
    if (i)
      near(
        y,
        groups[i - 1][0] +
          Math.max(...groups[i - 1][1].map((n) => n.bounds.height)) +
          16,
      );
  }
  if (text)
    assert.ok(
      scene.nodes
        .filter((n) => n.kind === "text")
        .every((n) => n.lines.length >= 2),
    );
}
