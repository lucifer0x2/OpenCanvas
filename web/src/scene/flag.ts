/** Explicit query selection wins over a saved preference, including ?scene=server. */
export function wasmSceneEnabled(
  search = globalThis.location?.search ?? "",
  storage?: Pick<Storage, "getItem">,
): boolean {
  const selected = new URLSearchParams(search).get("scene");
  if (selected !== null) return selected === "wasm";
  try {
    return (
      (storage ?? globalThis.localStorage)?.getItem("opencanvas.scene") ===
      "wasm"
    );
  } catch {
    return false;
  }
}
