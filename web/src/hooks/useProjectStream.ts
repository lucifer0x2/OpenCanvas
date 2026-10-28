import { useEffect, useRef, useState } from "react";
import { projectPath } from "../api";
import type {
  ActivityEvent,
  CommitEvent,
  Connection,
  ProjectRef,
} from "../types";

export interface StreamHandlers {
  /** Revision of the last accepted snapshot of the streamed project, for the replay cursor. */
  cursor: () => number;
  onOpen: () => void;
  /** Null when the event data could not be parsed. */
  onCommit: (event: CommitEvent | null) => void;
  onReset: () => void;
  onCreated: (project: ProjectRef) => void;
  onActivity: (event: ActivityEvent) => void;
}

const WORKSPACE_EVENTS = "/api/v1/events";

function parse<T>(message: Event): T | null {
  try {
    return JSON.parse((message as MessageEvent).data) as T;
  } catch {
    return null;
  }
}

/**
 * The tab's single `EventSource`: the project stream while a project is selected, the
 * workspace stream while none is. The previous stream closes before the next one opens, and a
 * failed stream is closed before the backoff timer opens its replacement.
 */
export function useProjectStream(
  projectId: string,
  reload: number,
  handlers: StreamHandlers,
): Connection {
  const [connection, setConnection] = useState<Connection>("connecting");
  const latest = useRef(handlers);
  useEffect(() => {
    latest.current = handlers;
  });

  useEffect(() => {
    let stopped = false;
    let stream: EventSource | undefined;
    let timer: ReturnType<typeof setTimeout>;
    let attempts = 0;
    setConnection("connecting");
    const url = () => {
      if (!projectId) return WORKSPACE_EVENTS;
      // Reopen with the last HTTP snapshot cursor, rather than an unverified event ID.
      const cursor = `${projectId}:${latest.current.cursor()}`;
      return `${projectPath(projectId)}/events?cursor=${encodeURIComponent(cursor)}`;
    };
    const connect = () => {
      if (stopped) return;
      stream = new EventSource(url());
      stream.onopen = () => {
        attempts = 0;
        setConnection("live");
        // Hints are not durable: reconcile the list and snapshot on every open.
        latest.current.onOpen();
      };
      stream.addEventListener("commit", (message) =>
        latest.current.onCommit(parse<CommitEvent>(message)),
      );
      stream.addEventListener("reset_required", () => latest.current.onReset());
      stream.addEventListener("project_created", (message) => {
        const project = parse<ProjectRef>(message);
        if (project?.projectId) latest.current.onCreated(project);
      });
      stream.addEventListener("project_activity", (message) => {
        const event = parse<ActivityEvent>(message);
        if (event?.projectId) latest.current.onActivity(event);
      });
      stream.onerror = () => {
        stream?.close();
        if (stopped) return;
        setConnection("reconnecting");
        timer = setTimeout(
          connect,
          Math.min(8000, 500 * 2 ** attempts++) + Math.random() * 250,
        );
      };
    };
    connect();
    return () => {
      stopped = true;
      stream?.close();
      clearTimeout(timer);
    };
  }, [projectId, reload]);
  return connection;
}
