/** The preview URL: `/preview?project=<projectId>&frame=<frameId>[&width=&height=]`. */
export interface PreviewRoute {
  projectId: string;
  frameId: string;
  width?: number;
  height?: number;
}

export const PREVIEW_PATH = "/preview";

export const isPreviewPath = (pathname: string) =>
  pathname === PREVIEW_PATH || pathname.startsWith(`${PREVIEW_PATH}/`);

const size = (value: string | null) => {
  const number = Number(value);
  return value && Number.isFinite(number) && number > 0 ? number : undefined;
};

export function readRoute(search: string): PreviewRoute {
  const params = new URLSearchParams(search);
  return {
    projectId: params.get("project") ?? "",
    frameId: params.get("frame") ?? "",
    width: size(params.get("width")),
    height: size(params.get("height")),
  };
}

export function routeUrl(route: PreviewRoute): string {
  const params = new URLSearchParams({ project: route.projectId });
  if (route.frameId) params.set("frame", route.frameId);
  if (route.width) params.set("width", String(route.width));
  if (route.height) params.set("height", String(route.height));
  return `${PREVIEW_PATH}?${params}`;
}

/** History entry state: how many preview navigations lie behind this entry. */
export interface PreviewHistory {
  ocPreviewDepth: number;
}

export function historyDepth(state: unknown): number {
  const depth = (state as PreviewHistory | null)?.ocPreviewDepth;
  return typeof depth === "number" ? depth : 0;
}
