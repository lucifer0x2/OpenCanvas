import { useEffect, type ReactNode } from "react";
import { ViewerPasswordDialog } from "./ViewerPasswordDialog";
import { useViewerSession } from "./useViewerSession";
import "./viewer-auth.css";

/** Both viewer routes stay unmounted until the server confirms access. */
export function ViewerGate({ children }: { children: ReactNode }) {
  const { status, retry, unlocked } = useViewerSession();
  useEffect(() => {
    if (status === "locked") document.title = "OpenCanvas";
  }, [status]);

  if (status === "open") return children;
  if (status === "locked") return <ViewerPasswordDialog onUnlock={unlocked} />;
  return (
    <main className="viewer-access-status">
      {status === "checking" ? (
        <p role="status">Checking viewer access…</p>
      ) : (
        <>
          <p role="alert">Could not check viewer access.</p>
          <button type="button" onClick={retry}>
            Try again
          </button>
        </>
      )}
    </main>
  );
}
