import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import { PreviewApp } from "./preview/PreviewApp";
import { isPreviewPath } from "./preview/route";
import "./styles.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    {isPreviewPath(location.pathname) ? <PreviewApp /> : <App />}
  </StrictMode>,
);
