/** WASM is the default; recognized queries override a saved troubleshooting preference. */
export function wasmSceneEnabled(
  search = globalThis.location?.search ?? "",
  storage?: Pick<Storage, "getItem">,
): boolean {
  const selected = new URLSearchParams(search).get("scene");
  if (selected === "wasm") return true;
  if (selected === "server") return false;
  try {
    return (
      (storage ?? globalThis.localStorage)?.getItem("opencanvas.scene") !==
      "server"
    );
  } catch {
    return true;
  }
}
