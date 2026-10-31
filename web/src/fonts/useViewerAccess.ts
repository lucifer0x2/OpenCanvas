import { useEffect, useState } from "react";
import { read } from "../api";

/** Listener capability is advisory UI state; the public server independently rejects writes. */
export function useViewerAccess() {
  const [readOnly, setReadOnly] = useState(true);
  useEffect(() => {
    const controller = new AbortController();
    read<{ readOnly: boolean }>("/api/v1/viewer", controller.signal)
      .then((access) => setReadOnly(access.readOnly !== false))
      .catch(() => {});
    return () => controller.abort();
  }, []);
  return readOnly;
}
