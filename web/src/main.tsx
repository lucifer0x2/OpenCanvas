import { lazy, StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { ViewerGate } from "./auth/ViewerGate";
import { RouteBoundary } from "./app/RouteBoundary";
import { isPreviewPath } from "./preview/route";
import { SessionDataProvider } from "./data/SessionDataProvider";
import "./styles.css";

const App = lazy(() =>
  import("./App").then((module) => ({ default: module.App })),
);
const PreviewApp = lazy(() =>
  import("./preview/PreviewApp").then((module) => ({
    default: module.PreviewApp,
  })),
);

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <ViewerGate>
      <SessionDataProvider>
        <RouteBoundary>
          {isPreviewPath(location.pathname) ? <PreviewApp /> : <App />}
        </RouteBoundary>
      </SessionDataProvider>
    </ViewerGate>
  </StrictMode>,
);
