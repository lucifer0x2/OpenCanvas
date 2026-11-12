import { useEffect, useState, type ReactNode } from "react";
import { SWRConfig, useSWRConfig } from "swr";
import { snapshotResource } from "./snapshotResource";

function SnapshotLifetime({ children }: { children: ReactNode }) {
  const { cache, mutate } = useSWRConfig();
  useEffect(() => snapshotResource(cache, mutate).attach(), [cache, mutate]);
  return children;
}

/** The cache belongs to the authorized route lifetime, never a global session. */
export function SessionDataProvider({ children }: { children: ReactNode }) {
  const [cache] = useState(() => new Map());
  return (
    <SWRConfig value={{ provider: () => cache, shouldRetryOnError: false }}>
      <SnapshotLifetime>{children}</SnapshotLifetime>
    </SWRConfig>
  );
}
