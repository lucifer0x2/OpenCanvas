import { StrictMode, useCallback, useState } from "react";
import { createRoot } from "react-dom/client";
import { useSWRConfig } from "swr";
import { useAcceptedSnapshot, useSnapshot } from "../src/hooks/useSnapshot";
import { SessionDataProvider } from "../src/data/SessionDataProvider";
import { nodeIndex, snapshotNodeIndex, type NodeIndex } from "../src/nodeIndex";
import { reconcileSelection } from "../src/state";
import type { Snapshot } from "../src/types";

let index: NodeIndex | undefined;
let sequence = 0;
const identities = new WeakMap<NodeIndex, number>();
const accepted: { projectId: string; revision: number; index: number; primed: boolean; selected: string }[] = [];
let current: ReturnType<typeof useSnapshot>;
let selectProject: (value: string) => void;
let payloadCount: () => number;

function Probe() {
  const { cache } = useSWRConfig();
  payloadCount = () => [...cache.keys()].filter((key) => cache.get(key)?.data?.snapshot).length;
  const [projectId, setProjectId] = useState("a");
  selectProject = setProjectId;
  current = useSnapshot(projectId);
  const accept = useCallback((next: Snapshot) => {
    // If the hook did not prime before calling us, this clone would build a second index.
    const cloned = snapshotNodeIndex({ ...next, nodes: structuredClone(next.nodes) });
    const primed = nodeIndex(next.nodes) === cloned;
    const selection = reconcileSelection(next, {
      projectId: next.projectId, pageId: "page", frameId: "frame", nodeId: "node",
    });
    if (!identities.has(cloned)) identities.set(cloned, ++sequence);
    accepted.push({ projectId: next.projectId, revision: next.revision,
      index: identities.get(cloned)!, primed, selected: selection.nodeId });
  }, []);
  useAcceptedSnapshot(current.snapshot, current.validation, accept);
  index = current.snapshot ? snapshotNodeIndex(current.snapshot) : undefined;
  return <output>{projectId}:{current.snapshot?.revision}</output>;
}

createRoot(document.getElementById("root")!).render(<StrictMode><SessionDataProvider><Probe /></SessionDataProvider></StrictMode>);
Object.assign(window, { snapshotIndexProbe: {
  refresh: () => current.refresh(),
  select: (projectId: string) => selectProject(projectId),
  value: () => ({ accepted, snapshot: current.snapshot, text: index?.byId.get("node")?.props.text, payloads: payloadCount() }),
} });
