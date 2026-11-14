import useSWR from "swr";
import { read } from "../api";

/** Listener capability is shared within the current viewer session's cache. */
export function useViewerAccess() {
  const { data, error } = useSWR(
    "/api/v1/viewer",
    (path) => read<{ readOnly: boolean }>(path),
    {
      revalidateIfStale: false,
      revalidateOnFocus: false,
      revalidateOnReconnect: false,
      shouldRetryOnError: false,
    },
  );
  return error ? true : data ? data.readOnly !== false : null;
}
