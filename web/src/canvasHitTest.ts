import type { Scene } from "../../renderer/types";

export function hitTest(scene: Scene, x: number, y: number) {
  const byId = new Map(scene.nodes.map((node) => [node.id, node]));
  const inside = (b: { x: number; y: number; width: number; height: number }) =>
    x >= b.x && y >= b.y && x <= b.x + b.width && y <= b.y + b.height;
  return [...scene.nodes].reverse().find((node) => {
    if (!inside(node.bounds)) return false;
    let parent = byId.get(node.parentId ?? "");
    let guard = 0;
    while (parent && guard++ <= scene.nodes.length) {
      if (parent.clip && !inside(parent.bounds)) return false;
      parent = byId.get(parent.parentId ?? "");
    }
    return true;
  });
}
