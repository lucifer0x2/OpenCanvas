import { useCallback, useEffect, useRef, useState } from "react";
import {
  clearActivity,
  commitAction,
  followTarget,
  markActivity,
} from "../state";
import { useFollowAgent } from "./useFollowAgent";
import { useProjectStream } from "./useProjectStream";
import type { useProjectList } from "./useProjectList";
import type { useSnapshot } from "./useSnapshot";
import type { useProject } from "./useProject";

/** One live subscription, the follow preference and unseen activity. No server-data copy. */
export function useProjectLive(
  list: ReturnType<typeof useProjectList>,
  navigation: ReturnType<typeof useProject>,
  snapshot: ReturnType<typeof useSnapshot>,
) {
  const projectId = navigation.selection.projectId;
  const [reload, setReload] = useState(0);
  const [activity, setActivity] = useState<string[]>([]);
  const [follow, setFollow] = useFollowAgent();
  const followGeneration = useRef(0);
  useEffect(
    () => () => {
      followGeneration.current++;
    },
    [follow, projectId, reload],
  );
  const selectProject = useCallback(
    (id: string) => {
      setActivity((marked) => clearActivity(marked, id));
      navigation.selectProject(id);
    },
    [navigation.selectProject],
  );
  const hinted = (
    id: string,
    name: string,
    created: boolean,
    membership?: ReturnType<typeof list.refetch>,
  ) => {
    if (followTarget(follow, id, projectId)) {
      const navigate = () => {
        selectProject(id);
        if (projectId) navigation.setNotice(`Following the agent to ${name}.`);
      };
      if (list.projects.some((project) => project.projectId === id)) navigate();
      else {
        const generation = ++followGeneration.current;
        const href = window.location.href;
        void (membership ?? list.refetch()).then((projects) => {
          if (
            generation !== followGeneration.current ||
            window.location.href !== href
          )
            return;
          if (projects?.some((project) => project.projectId === id)) navigate();
          else
            navigation.setNotice(
              "The linked or selected project is unavailable. Keeping the current project.",
            );
        });
      }
    } else if (id !== projectId) {
      if (!created)
        setActivity((marked) => markActivity(marked, id, projectId));
      navigation.setNotice(
        created
          ? `${name} was created. Select it in Project to view it.`
          : `${name} has new changes. Select it in Project to view them.`,
      );
    }
  };
  const connection = useProjectStream(projectId, reload, {
    cursor: () => snapshot.revision() ?? 0,
    onOpen: () => {
      if (list.loaded || !list.loading) list.refetch();
    },
    onCommit: (event) => {
      if (!event) {
        void snapshot.refresh();
        return;
      }
      const action = commitAction(event, projectId, snapshot.revision() ?? -1);
      if (action === "gap") navigation.setNotice("Catching up with changes…");
      if (action !== "ignore") list.bump(event);
    },
    onReset: () => {
      if (projectId)
        navigation.setNotice("Refreshing after an event history reset…");
      list.refetch();
      void snapshot.refresh();
    },
    onVisibilityChanged: () => {
      navigation.setNotice("");
      list.refetch();
      void snapshot.refresh(true);
    },
    onReconnect: () => {
      followGeneration.current++;
      void list.refetch();
    },
    onCreated: (project) => {
      const membership = list.refetch();
      hinted(project.projectId, project.name, true, membership);
    },
    onActivity: (event) => {
      list.bump(event);
      hinted(
        event.projectId,
        list.projects.find((project) => project.projectId === event.projectId)
          ?.name ?? event.projectId,
        false,
      );
    },
  });
  const retry = useCallback(() => {
    followGeneration.current++;
    list.refetch();
    void snapshot.refresh();
    setReload((value) => value + 1);
  }, [list.refetch, snapshot.refresh]);
  return { connection, activity, follow, setFollow, selectProject, retry };
}
