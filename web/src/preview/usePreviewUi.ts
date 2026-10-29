import { useCallback, useEffect, useRef, useState } from "react";

const QUERY = "(prefers-reduced-motion: reduce)";

/** Starts from the OS `prefers-reduced-motion` setting and follows it until toggled. */
export function useReducedMotion() {
  const [reduced, setReduced] = useState(() => matchMedia(QUERY).matches);
  const overridden = useRef(false);
  useEffect(() => {
    const media = matchMedia(QUERY);
    const follow = () => {
      if (!overridden.current) setReduced(media.matches);
    };
    media.addEventListener("change", follow);
    return () => media.removeEventListener("change", follow);
  }, []);
  const toggle = useCallback((next: boolean) => {
    overridden.current = true;
    setReduced(next);
  }, []);
  return [reduced, toggle] as const;
}

/** A short message that hides itself after a few seconds. */
export function useToast(timeoutMs = 3500) {
  const [toast, setToast] = useState("");
  const timer = useRef<ReturnType<typeof setTimeout>>(undefined);
  const show = useCallback(
    (message: string) => {
      clearTimeout(timer.current);
      setToast(message);
      timer.current = setTimeout(() => setToast(""), timeoutMs);
    },
    [timeoutMs],
  );
  useEffect(() => () => clearTimeout(timer.current), []);
  return { toast, show };
}
