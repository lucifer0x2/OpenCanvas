import { useCallback, useMemo, useState } from "react";
import { CanvasView } from "../components/CanvasView";
import { Inspector } from "../components/Inspector";
import { Layers } from "../components/Layers";
import { NodeActionsProvider } from "../components/NodeActionsProvider";
import { PRESETS } from "../components/ScopeControls";
import { selectionNode } from "../deepLink";
import { ALL_FRAMES, frameDimensions, frameForNode } from "../frameScope";
import { usePageScenes } from "../hooks/usePageScenes";
import type { ProjectView } from "./projectView";
import { useScene } from "../hooks/useScene";
import { currentScene, shownRevision } from "../scene/staleFrames";
import { nodeName } from "../state";
import type { DesignNode, SceneRequest } from "../types";
import { ViewerStatus } from "./ViewerStatus";

interface Props {
  project: ProjectView;
  nodes: DesignNode[];
  frames: DesignNode[];
  frame?: DesignNode;
  selectedFrame?: DesignNode;
  preset: string;
  leftOpen: boolean;
  rightOpen: boolean;
  showNotice: boolean;
}

/** Own scene subscriptions at the canvas boundary so frame progress does not rerender chrome. */
export function SceneWorkspace({
  project,
  nodes,
  frames,
  frame,
  selectedFrame,
  preset,
  leftOpen,
  rightOpen,
  showNotice,
}: Props) {
  const { snapshot, selection, selectScope } = project;
  const [retryKey, setRetryKey] = useState(0);
  const retryScene = useCallback(() => setRetryKey((key) => key + 1), []);
  const frameIds = useMemo(() => frames.map((entry) => entry.id), [frames]);
  const selectedNode = useMemo(
    () => selectionNode(nodes, selection.nodeId),
    [nodes, selection.nodeId],
  );
  const all = selection.frameId === ALL_FRAMES;
  const viewport = PRESETS.find((entry) => entry.key === preset) ?? PRESETS[0];
  const request: SceneRequest | null =
    snapshot && frame
      ? {
          projectId: snapshot.projectId,
          revision: snapshot.revision,
          frameId: frame.id,
          width: viewport.width || frameDimensions(frame).width,
          height: viewport.height || frameDimensions(frame).height,
        }
      : null;
  const singleView = useScene(request, retryKey, preset, snapshot);
  const pageView = usePageScenes(
    all ? snapshot : null,
    selection.pageId,
    frames,
    preset,
    viewport,
    retryKey,
  );
  const view = all ? pageView : singleView;
  // Computed geometry only from a scene answering exactly the current snapshot.
  const inspected = all
    ? currentScene(
        view.frames?.find((entry) => entry.id === selectedFrame?.id),
        snapshot?.revision,
      )
    : shownRevision(view) === snapshot?.revision
      ? view.scene
      : undefined;
  const inspectorScene =
    inspected?.provenance.projectId === snapshot?.projectId
      ? inspected
      : undefined;
  const selectNode = useCallback(
    (nodeId: string) => selectScope({ nodeId }),
    [selectScope],
  );
  const resolveSelection = useCallback(
    (id: string) => ({
      ...selection,
      nodeId: id,
      frameId:
        frameForNode(nodes, id, selection.pageId)?.id ?? selection.frameId,
    }),
    [nodes, selection],
  );

  return (
    <NodeActionsProvider
      selection={selection}
      revision={snapshot?.revision}
      select={selectNode}
      resolveSelection={resolveSelection}
    >
      <div className="workspace">
        {leftOpen && (
          <aside className="left-panel">
            <Layers
              nodes={nodes}
              frameId={selection.frameId}
              frameIds={all ? frameIds : undefined}
              selectedId={selection.nodeId}
              select={selectNode}
            />
          </aside>
        )}
        <CanvasView
          view={view}
          selectedId={selection.nodeId}
          select={selectNode}
          retry={retryScene}
          frameName={
            all ? `All frames · ${frames.length}` : frame ? nodeName(frame) : ""
          }
        />
        {rightOpen && (
          <Inspector
            node={selectedNode}
            scene={inspectorScene}
            revision={snapshot?.revision}
          />
        )}
      </div>
      <ViewerStatus project={project} view={view} showNotice={showNotice} />
    </NodeActionsProvider>
  );
}
