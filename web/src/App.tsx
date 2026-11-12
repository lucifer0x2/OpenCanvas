import { useCallback, useMemo, useState } from "react";
import { SceneWorkspace } from "./app/SceneWorkspace";
import { ScopeControls } from "./components/ScopeControls";
import { Topbar } from "./components/Topbar";
import { useProject } from "./hooks/useProject";
import { useProjectList } from "./hooks/useProjectList";
import { useProjectLive } from "./hooks/useProjectLive";
import { useAcceptedSnapshot, useSnapshot } from "./hooks/useSnapshot";
import { frameForNode } from "./frameScope";
import { EMPTY_NODES, nodeIndex, snapshotNodeIndex } from "./nodeIndex";
import useLocalStorage from "./hooks/ultility/useLocalStorage";

const isBoolean = (value: unknown): value is boolean =>
  typeof value === "boolean";

/** Project navigation and shell preferences; scene progress stays in the workspace. */
export function App() {
  const list = useProjectList();
  const navigation = useProject(list.projects, list.loaded);
  const { selection } = navigation;
  const knownRevision =
    list.projects.find((entry) => entry.projectId === selection.projectId)
      ?.revision ?? null;
  const snapshotQuery = useSnapshot(selection.projectId, knownRevision);
  const { snapshot } = snapshotQuery;
  useAcceptedSnapshot(snapshot, snapshotQuery.validation, navigation.accepted);
  const live = useProjectLive(list, navigation, snapshotQuery);
  const selectScope = useCallback(
    (patch: Parameters<typeof navigation.selectScope>[1]) =>
      navigation.selectScope(snapshot, patch),
    [navigation.selectScope, snapshot],
  );
  const project = {
    snapshot,
    selection,
    selectScope,
    notice: navigation.notice,
    error: list.error || snapshotQuery.error,
    ...live,
  };
  const [leftOpen, setLeftOpen] = useLocalStorage(
    "left-open",
    true,
    (v) => typeof v == "boolean",
  );
  const [rightOpen, setRightOpen] = useLocalStorage(
    "right-open",
    true,
    (v) => typeof v == "boolean",
  );
  const [preset, setPreset] = useState("frame");
  const [isShowNoticeBar, setShowNoticeBar] = useLocalStorage(
    "is_show_notice_bar",
    false,
    isBoolean,
  );
  const nodes = snapshot?.nodes ?? EMPTY_NODES;
  const index = snapshot ? snapshotNodeIndex(snapshot) : nodeIndex(EMPTY_NODES);
  const pages = useMemo(
    () =>
      (index.childrenByParent.get(null) ?? EMPTY_NODES).filter(
        (node) => node.kind === "page",
      ),
    [index],
  );
  const frames = useMemo(
    () =>
      (index.childrenByParent.get(selection.pageId) ?? EMPTY_NODES).filter(
        (node) => node.kind === "frame",
      ),
    [index, selection.pageId],
  );
  const frame = index.byId.get(selection.frameId);
  const selectedFrame = useMemo(
    () => frameForNode(nodes, selection.nodeId, selection.pageId),
    [nodes, selection.nodeId, selection.pageId],
  );
  const toggleLeft = useCallback(() => setLeftOpen(!leftOpen), [leftOpen]);
  const toggleRight = useCallback(() => setRightOpen(!rightOpen), [rightOpen]);

  return (
    <div
      className={`app-shell ${leftOpen ? "left-open" : ""} ${rightOpen ? "right-open" : ""}`}
    >
      <Topbar
        projects={list.projects}
        projectId={selection.projectId}
        activity={project.activity}
        loading={list.loading}
        failed={!!project.error}
        selectProject={project.selectProject}
        revision={
          snapshot?.projectId === selection.projectId
            ? snapshot.revision
            : undefined
        }
        refresh={project.retry}
        frameId={frame?.id ?? selectedFrame?.id ?? ""}
        leftOpen={leftOpen}
        rightOpen={rightOpen}
        toggleLeft={toggleLeft}
        toggleRight={toggleRight}
        isShowNoticeBar={isShowNoticeBar}
        setShowNoticeBar={setShowNoticeBar}
      />
      <ScopeControls
        pages={pages}
        frames={frames}
        selection={selection}
        select={project.selectScope}
        preset={preset}
        setPreset={setPreset}
      />
      <SceneWorkspace
        project={project}
        nodes={nodes}
        frames={frames}
        frame={frame}
        selectedFrame={selectedFrame}
        preset={preset}
        leftOpen={leftOpen}
        rightOpen={rightOpen}
        showNotice={isShowNoticeBar}
      />
    </div>
  );
}
