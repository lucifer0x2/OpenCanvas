import { StrictMode, useState } from "react";
import { createRoot } from "react-dom/client";
import { SWRConfig } from "swr";
import { FontManager } from "../src/fonts/FontManager";

function Fixture() {
  const [projectId, setProjectId] = useState("p");
  const [revision, setRevision] = useState(0);
  Object.assign(window, { selectFontProject: setProjectId, selectFontRevision: setRevision });
  return <FontManager key={projectId} projectId={projectId} projectName={projectId} revision={revision} changed={() => {}} />;
}

createRoot(document.getElementById("root")!).render(
  <StrictMode><SWRConfig value={{ provider: () => new Map() }}><Fixture /></SWRConfig></StrictMode>,
);
