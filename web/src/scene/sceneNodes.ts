import type { Scene } from "../../../renderer/types";

const indexes = new WeakMap<Scene, ReadonlyMap<string, Scene["nodes"][number]>>();

/** Scene identities survive board revalidation; selection and zoom reuse their lookup. */
export function sceneNodes(scene: Scene) {
  let index = indexes.get(scene);
  if (!index) {
    index = new Map(scene.nodes.map((node) => [node.id, node]));
    indexes.set(scene, index);
  }
  return index;
}
