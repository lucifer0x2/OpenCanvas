import { useCallback, useState } from "react";

/**
 * Whether the viewer switches to the project an agent just created or changed. Off by default:
 * jumping away from the project being viewed loses context. Change only this constant to flip it.
 */
export const FOLLOW_AGENT_DEFAULT = false;
const KEY = "opencanvas.followAgent";

function readFollow() {
  try {
    const stored = localStorage.getItem(KEY);
    return stored === null ? FOLLOW_AGENT_DEFAULT : stored === "true";
  } catch {
    return FOLLOW_AGENT_DEFAULT;
  }
}

export function useFollowAgent() {
  const [follow, setFollow] = useState(readFollow);
  const change = useCallback((next: boolean) => {
    setFollow(next);
    try {
      localStorage.setItem(KEY, String(next));
    } catch {
      /* The switch still works for this page without storage. */
    }
  }, []);
  return [follow, change] as const;
}
