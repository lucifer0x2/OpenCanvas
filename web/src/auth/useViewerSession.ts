import { useEffect, useState } from "react";
import { VIEWER_SESSION_CHECK } from "../api";
import { viewerAuth } from "./api";

type ViewerSession =
  | { status: "checking" | "unavailable" | "locked" }
  | { status: "open"; required: boolean };

const RECHECK_MS = 30_000;

export function useViewerSession() {
  const [session, setSession] = useState<ViewerSession>({ status: "checking" });
  const required = session.status === "open" && session.required;

  useEffect(() => {
    if (session.status !== "checking") return;
    const controller = new AbortController();
    viewerAuth(controller.signal)
      .then(({ authenticated, required }) => {
        if (controller.signal.aborted) return;
        setSession(
          authenticated ? { status: "open", required } : { status: "locked" },
        );
      })
      .catch(() => {
        if (!controller.signal.aborted) setSession({ status: "unavailable" });
      });
    return () => controller.abort();
  }, [session.status]);

  useEffect(() => {
    const open = session.status === "open";
    if (!open && session.status !== "locked") return;
    const controller = new AbortController();
    let pending = false;
    const recheck = async () => {
      if (pending) return;
      pending = true;
      try {
        const result = await viewerAuth(controller.signal);
        if (controller.signal.aborted) return;
        if (!result.authenticated) {
          if (open) setSession({ status: "locked" });
        } else if (!open || result.required !== required) {
          setSession({ status: "open", required: result.required });
        }
      } catch {
        // Keep the loaded view during connectivity issues. The server still gates new data.
      } finally {
        pending = false;
      }
    };
    const visible = () => {
      if (document.visibilityState === "visible") void recheck();
    };
    const restored = (event: PageTransitionEvent) => {
      if (event.persisted) void recheck();
    };
    window.addEventListener(VIEWER_SESSION_CHECK, recheck);
    window.addEventListener("pageshow", restored);
    if (required || !open)
      document.addEventListener("visibilitychange", visible);
    const timer = required
      ? window.setInterval(visible, RECHECK_MS)
      : undefined;
    return () => {
      controller.abort();
      window.removeEventListener(VIEWER_SESSION_CHECK, recheck);
      window.removeEventListener("pageshow", restored);
      document.removeEventListener("visibilitychange", visible);
      window.clearInterval(timer);
    };
  }, [session.status, required]);

  return {
    status: session.status,
    retry: () => setSession({ status: "checking" }),
    unlocked: (required: boolean) => setSession({ status: "open", required }),
  };
}
