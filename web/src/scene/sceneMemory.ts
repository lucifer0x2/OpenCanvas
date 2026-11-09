import type { Scene } from "../../../renderer/types";

/** Recently used decorated scenes by frame content key; the tab's first cache tier. */
export class SceneMemory {
  private entries = new Map<string, Scene>();

  constructor(private limit = 96) {}

  get(key: string): Scene | undefined {
    const scene = key ? this.entries.get(key) : undefined;
    if (scene) {
      this.entries.delete(key);
      this.entries.set(key, scene);
    }
    return scene;
  }

  set(key: string, scene: Scene) {
    if (!key) return;
    this.entries.delete(key);
    this.entries.set(key, scene);
    while (this.entries.size > this.limit)
      this.entries.delete(this.entries.keys().next().value!);
  }

  remove(key: string) {
    this.entries.delete(key);
  }

  clear() {
    this.entries.clear();
  }
}

export const sceneMemory = new SceneMemory();
