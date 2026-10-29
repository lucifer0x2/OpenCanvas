import { useLayoutEffect, useState, type RefObject } from "react";

/** A fixed portal escapes toolbar overflow and follows scroll/resize. */
export function useSelectPosition(
  trigger: RefObject<HTMLButtonElement | null>,
  open: boolean,
) {
  const [position, setPosition] = useState({
    left: 8,
    top: 8,
    width: 240,
    maxHeight: 280,
    transform: "none",
  });
  useLayoutEffect(() => {
    if (!open || !trigger.current) return;
    const place = () => {
      const rect = trigger.current!.getBoundingClientRect();
      const width = Math.min(Math.max(rect.width, 240), window.innerWidth - 16);
      const below = window.innerHeight - rect.bottom - 12;
      const above = rect.top - 12;
      const upwards = below < 180 && above > below;
      const maxHeight = Math.max(40, Math.min(320, upwards ? above : below));
      setPosition({
        left: Math.max(8, Math.min(rect.left, window.innerWidth - width - 8)),
        top: upwards ? rect.top - 4 : rect.bottom + 4,
        transform: upwards ? "translateY(-100%)" : "none",
        width,
        maxHeight,
      });
    };
    place();
    const observer = new ResizeObserver(place);
    observer.observe(trigger.current);
    window.addEventListener("resize", place);
    window.addEventListener("scroll", place, true);
    return () => {
      observer.disconnect();
      window.removeEventListener("resize", place);
      window.removeEventListener("scroll", place, true);
    };
  }, [trigger, open]);
  return position;
}
