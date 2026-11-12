import { useSyncExternalStore } from "react";
import { navigationLink } from "../deepLink";
import type { Selection } from "../types";

const CHANGE = "opencanvas:navigation";
const currentHref = () => window.location.href;
function subscribe(changed: () => void) {
  window.addEventListener("popstate", changed);
  window.addEventListener(CHANGE, changed);
  return () => {
    window.removeEventListener("popstate", changed);
    window.removeEventListener(CHANGE, changed);
  };
}

/** Browser history owns navigation; React observes the current address directly. */
export function useNavigationUrl() {
  return useSyncExternalStore(subscribe, currentHref);
}

export function writeNavigation(
  selection: Selection,
  mode: "push" | "replace",
  expectedHref = currentHref(),
) {
  // A passive effect belonging to a previous address must never rewrite a popstate.
  if (currentHref() !== expectedHref) return currentHref();
  const href = navigationLink(expectedHref, selection);
  if (href !== expectedHref) {
    window.history[mode === "push" ? "pushState" : "replaceState"](
      window.history.state,
      "",
      href,
    );
    window.dispatchEvent(new Event(CHANGE));
  }
  return href;
}
