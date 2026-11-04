import { useEffect, useState } from "react";
import { read } from "../api";

/** Listener capability, with null while it is still being checked. */
export function useViewerAccess() {
  const [readOnly, setReadOnly] = useState<boolean | null>(null);
  useEffect(() => {
    const controller = new AbortController();
    read<{ readOnly: boolean }>("/api/v1/viewer", controller.signal)
      .then((access) => {
        if (!controller.signal.aborted) setReadOnly(access.readOnly !== false);
      })
      .catch(() => {
        if (!controller.signal.aborted) setReadOnly(true);
      });
    return () => controller.abort();
  }, []);
  return readOnly;
}
