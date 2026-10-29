import { routeUrl } from "../preview/route";

/** Opens the motion preview (`/preview`) of one frame in a new tab. */
export function PreviewButton(props: { projectId: string; frameId: string }) {
  const { projectId, frameId } = props;
  return (
    <button
      aria-label="Open motion preview of this frame in a new tab"
      disabled={!projectId || !frameId}
      onClick={() =>
        window.open(routeUrl({ projectId, frameId }), "_blank", "noopener")
      }
    >
      Preview
    </button>
  );
}
