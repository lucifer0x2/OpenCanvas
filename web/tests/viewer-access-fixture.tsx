import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { SWRConfig } from "swr";
import { useViewerAccess } from "../src/hooks/useViewerAccess";

function Access({ name }: { name: string }) {
  const access = useViewerAccess();
  return <output aria-label={name}>{access === null ? "pending" : access ? "read only" : "writable"}</output>;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode><SWRConfig value={{ provider: () => new Map() }}>
    <Access name="first" /><Access name="second" />
  </SWRConfig></StrictMode>,
);
