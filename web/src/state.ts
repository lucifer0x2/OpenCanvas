import type {
  ActivityEvent,
  CommitEvent,
  DesignNode,
  ProjectRef,
  RenderJob,
  SceneRequest,
  Selection,
  Snapshot,
} from "./types.ts";

export const EMPTY_SELECTION: Selection = {
  projectId: "",
  pageId: "",
  frameId: "",
  nodeId: "",
};

export function children(nodes: DesignNode[], parentId: string | null) {
  return nodes
    .filter((node) => node.parentId === parentId)
    .sort(
      (a, b) => Number(a.sortKey) - Number(b.sortKey) || a.id.localeCompare(b.id),
    );
}

export function nodeName(node?: DesignNode) {
  return typeof node?.props.name === "string"
    ? node.props.name
    : (node?.id ?? "Untitled");
}

export function reconcileSelection(
  snapshot: Snapshot,
  previous: Selection,
): Selection {
  const pages = children(snapshot.nodes, null).filter(
    (node) => node.kind === "page",
  );
  const page = pages.find((node) => node.id === previous.pageId) ?? pages[0];
  const frames = children(snapshot.nodes, page?.id ?? "").filter(
    (node) => node.kind === "frame",
  );
  const frame =
    frames.find((node) => node.id === previous.frameId) ?? frames[0];
  const byId = new Map(snapshot.nodes.map((node) => [node.id, node]));
  let candidate = byId.get(previous.nodeId);
  const seen = new Set<string>();
  while (candidate && candidate.id !== frame?.id && !seen.has(candidate.id)) {
    seen.add(candidate.id);
    candidate = byId.get(candidate.parentId ?? "");
  }
  const nodeId =
    candidate?.id === frame?.id
      ? previous.nodeId
      : (frame?.id ?? page?.id ?? "");
  return {
    projectId: snapshot.projectId,
    pageId: page?.id ?? "",
    frameId: frame?.id ?? "",
    nodeId,
  };
}

export function commitAction(
  event: CommitEvent,
  projectId: string,
  revision: number,
) {
  if (
    event.projectId !== projectId ||
    !Number.isSafeInteger(event.revision) ||
    event.revision <= revision
  )
    return "ignore";
  return event.revision === revision + 1 ? "refresh" : "gap";
}

export function acceptsJob(
  job: RenderJob,
  request: SceneRequest,
  expectedJobId?: string,
  state = "default",
) {
  if (expectedJobId && job.jobId !== expectedJobId) return false;
  if (
    job.projectId !== request.projectId ||
    job.revision !== request.revision ||
    job.frameId !== request.frameId
  )
    return false;
  if (job.status !== "ready") return true;
  const provenance = job.scene?.provenance;
  return (
    !!provenance &&
    provenance.projectId === request.projectId &&
    provenance.revision === request.revision &&
    provenance.frameId === request.frameId &&
    provenance.viewport.width === request.width &&
    provenance.viewport.height === request.height &&
    provenance.theme === "default" &&
    provenance.state === state &&
    (!job.sceneKey || job.sceneKey === provenance.sourceSceneKey)
  );
}

export function readSelection(): Selection {
  try {
    return {
      ...EMPTY_SELECTION,
      ...JSON.parse(localStorage.getItem("opencanvas.selection") ?? "{}"),
    };
  } catch {
    return EMPTY_SELECTION;
  }
}

export function saveSelection(selection: Selection) {
  try {
    localStorage.setItem("opencanvas.selection", JSON.stringify(selection));
  } catch {
    /* Viewing still works without storage. */
  }
}

/** Union by projectId, keeping the newest revision, in the list API's slug order. */
export function mergeProjects(
  current: ProjectRef[],
  incoming: ProjectRef[],
): ProjectRef[] {
  const byId = new Map(current.map((entry) => [entry.projectId, entry]));
  for (const entry of incoming) {
    const known = byId.get(entry.projectId);
    byId.set(
      entry.projectId,
      known && known.revision > entry.revision
        ? { ...entry, revision: known.revision }
        : entry,
    );
  }
  return [...byId.values()].sort((a, b) =>
    a.slug < b.slug ? -1 : a.slug > b.slug ? 1 : 0,
  );
}

/** A `project_activity` hint only moves a listed project's revision forward. */
export function bumpRevision(
  projects: ProjectRef[],
  event: ActivityEvent,
): ProjectRef[] {
  if (!Number.isSafeInteger(event.revision)) return projects;
  let changed = false;
  const next = projects.map((entry) => {
    if (entry.projectId !== event.projectId || entry.revision >= event.revision)
      return entry;
    changed = true;
    return { ...entry, revision: event.revision };
  });
  return changed ? next : projects;
}

/** Projects with unseen agent changes; the one being viewed is never marked. */
export function markActivity(
  marked: string[],
  projectId: string,
  viewing: string,
): string[] {
  return !projectId || projectId === viewing || marked.includes(projectId)
    ? marked
    : [...marked, projectId];
}

export function clearActivity(marked: string[], projectId: string) {
  return marked.includes(projectId)
    ? marked.filter((id) => id !== projectId)
    : marked;
}

/** Keeps a listed selection; otherwise the first project, as on the first load. */
export function pickProject(projects: ProjectRef[], current: string) {
  return projects.some((entry) => entry.projectId === current)
    ? current
    : (projects[0]?.projectId ?? "");
}

/**
 * Where a workspace hint moves the viewer: an empty workspace always takes a new project;
 * otherwise only the follow-agent switch moves away from the project being viewed.
 */
export function followTarget(
  follow: boolean,
  projectId: string,
  viewing: string,
): string | null {
  if (!projectId || projectId === viewing) return null;
  return follow || !viewing ? projectId : null;
}
