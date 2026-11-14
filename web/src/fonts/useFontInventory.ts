import useSWR, { useSWRConfig } from "swr";
import { listFonts } from "./api";

/** Visible revisions key the cache; the latest-inventory endpoint can return ahead. */
export function useFontInventory(
  projectId: string,
  revision: number | undefined,
  open: boolean,
) {
  const key = projectId && revision !== undefined
    ? ["font-inventory", projectId, revision] as const
    : null;
  const { mutate } = useSWRConfig();
  const { data, error, isValidating } = useSWR(
    open ? key : null,
    ([, id]) => listFonts(id),
    {
      keepPreviousData: false,
      revalidateIfStale: false,
      revalidateOnFocus: false,
      revalidateOnReconnect: false,
      shouldRetryOnError: false,
    },
  );
  const refresh = () => {
    // Clearing also invalidates a closed manager's cache for its next opening.
    if (key) void mutate(key, undefined, { revalidate: true, throwOnError: false });
  };
  return {
    data: data ?? null,
    loading: isValidating,
    error: error instanceof Error
      ? error.message
      : error ? "Could not load project fonts." : "",
    refresh,
  };
}
