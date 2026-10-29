// Scene geometry the Part D gate compares, same shape as the Rust `convert::geometry` dump:
// [[id, x, y, width, height, [[text, x, y, width], ...]], ...].
export const geometry = (scene) =>
  scene.nodes.map((n) => [n.id, n.bounds.x, n.bounds.y, n.bounds.width, n.bounds.height,
    n.lines.map((l) => [l.text, l.x, l.y, l.width])]);
