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
import { selectionNode } from "./deepLink.ts";
import { ALL_FRAMES, frameForNode } from "./frameScope.ts";
import { nodeIndex } from "./nodeIndex.ts";
import { inferPageId } from "./pageInference.ts";

export const EMPTY_SELECTION: Selection = {
  projectId: "",
  pageId: "",
  frameId: ALL_FRAMES,
  nodeId: "",
};

export function children(nodes: DesignNode[], parentId: string | null) {
  // Keep the public helper's independently mutable result.
  return [...(nodeIndex(nodes).childrenByParent.get(parentId) ?? [])];
}

function retainSelection(previous: Selection, next: Selection): Selection {
  return previous.projectId === next.projectId &&
    previous.pageId === next.pageId &&
    previous.frameId === next.frameId &&
    previous.nodeId === next.nodeId
    ? previous
    : next;
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
  // An explicit page wins; a partial link infers it from its frame or node.
  const pageId =
    previous.pageId ||
    (previous.projectId === snapshot.projectId
      ? inferPageId(snapshot.nodes, previous)
      : "");
  const page = pages.find((node) => node.id === pageId) ?? pages[0];
  const frames = children(snapshot.nodes, page?.id ?? "").filter(
    (node) => node.kind === "frame",
  );
  const frame = frames.find((node) => node.id === previous.frameId);
  if (!frame || previous.projectId !== snapshot.projectId) {
    return retainSelection(previous, {
      projectId: snapshot.projectId,
      pageId: page?.id ?? "",
      frameId: ALL_FRAMES,
      nodeId:
        previous.projectId === snapshot.projectId &&
        frameForNode(snapshot.nodes, previous.nodeId, page?.id ?? "")
          ? previous.nodeId
          : "",
    });
  }
  const { byId } = nodeIndex(snapshot.nodes);
  let candidate = selectionNode(snapshot.nodes, previous.nodeId);
  const seen = new Set<string>();
  while (candidate && candidate.id !== frame?.id && !seen.has(candidate.id)) {
    seen.add(candidate.id);
    candidate =
      byId.get(candidate.parentId ?? "") ??
      selectionNode(snapshot.nodes, candidate.parentId ?? "");
  }
  const nodeId =
    previous.nodeId === ""
      ? ""
      : candidate?.id === frame?.id
        ? previous.nodeId
        : (frame?.id ?? page?.id ?? "");
  return retainSelection(previous, {
    projectId: snapshot.projectId,
    pageId: page?.id ?? "",
    frameId: frame?.id ?? "",
    nodeId,
  });
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
    (request.hugHeight || provenance.viewport.height === request.height) &&
    provenance.theme === "default" &&
    provenance.state === state &&
    (!job.sceneKey || job.sceneKey === provenance.sourceSceneKey)
  );
}

/** Union by projectId, keeping the newest revision, in the list API's slug order. */
export function mergeProjects(
  current: ProjectRef[],
  incoming: ProjectRef[],
): ProjectRef[] {
  const byId = new Map(current.map((entry) => [entry.projectId, entry]));
  for (const entry of incoming) {
    const known = byId.get(entry.projectId);
    const revision = Math.max(
      known?.revision ?? entry.revision,
      entry.revision,
    );
    const same =
      known &&
      known.slug === entry.slug &&
      known.name === entry.name &&
      known.revision === revision;
    byId.set(entry.projectId, same ? known : { ...entry, revision });
  }
  const next = [...byId.values()].sort((a, b) =>
    a.slug < b.slug ? -1 : a.slug > b.slug ? 1 : 0,
  );
  return next.length === current.length &&
    next.every((entry, i) => entry === current[i])
    ? current
    : next;
}

/** HTTP listing is authoritative for membership; live hints can retain newer revisions. */
export function reconcileProjects(
  current: ProjectRef[],
  incoming: ProjectRef[],
) {
  const visible = new Set(incoming.map(({ projectId }) => projectId));
  const retained = current.every(({ projectId }) => visible.has(projectId))
    ? current
    : current.filter(({ projectId }) => visible.has(projectId));
  return mergeProjects(retained, incoming);
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
