import {
  createContext, useCallback, useContext, useEffect, useLayoutEffect,
  useMemo, useRef, useState,
} from "react";
import type { ReactNode } from "react";
import type { Selection } from "../types";
import { nodeLink } from "../deepLink";
import { exportNode } from "../nodeExport";
import "../node-actions.css";

type Position = { x: number; y: number; id: string };
interface Actions {
  open: (id: string, x: number, y: number) => void;
  copy: (id?: string) => void;
  image: (format: "png" | "svg", id?: string) => void;
}
const Context = createContext<Actions | null>(null);
const BusyContext = createContext(false);
export const useNodeActions = () => useContext(Context);
export const useNodeActionBusy = () => useContext(BusyContext);

export function NodeActionsProvider({
  children,
  selection,
  revision,
  select,
  resolveSelection,
}: {
  children: ReactNode;
  selection: Selection;
  revision?: number;
  select: (id: string) => void;
  resolveSelection?: (id: string) => Selection;
}) {
  const [menu, setMenu] = useState<Position | null>(null);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const active = useRef(false);
  // Commands retain their identity while reading the latest committed selection.
  // Each export captures its project/revision before any asynchronous work.
  const current = useRef({ selection, revision, select, resolveSelection });
  useLayoutEffect(() => {
    current.current = { selection, revision, select, resolveSelection };
  }, [selection, revision, select, resolveSelection]);
  const copy = useCallback(async (id?: string) => {
    const { selection, resolveSelection } = current.current;
    const nodeId = id ?? selection.nodeId;
    setMenu(null);
    try {
      await navigator.clipboard.writeText(
        nodeLink(
          location.href,
          resolveSelection?.(nodeId) ?? { ...selection, nodeId },
        ),
      );
      setMessage(
        "Link copied. Paste it into your AI chat to identify this element.",
      );
    } catch {
      setMessage(
        "Could not copy the link. Allow clipboard access and try again.",
      );
    }
  }, []);
  const image = useCallback(async (format: "png" | "svg", id?: string) => {
    const { selection, revision } = current.current;
    setMenu(null);
    if (active.current || revision === undefined) return;
    active.current = true;
    setBusy(true);
    setMessage(`Exporting ${format.toUpperCase()}…`);
    try {
      await exportNode(
        selection.projectId, id ?? selection.nodeId, revision, format,
      );
      setMessage(`${format.toUpperCase()} downloaded.`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Export failed.");
    } finally {
      active.current = false;
      setBusy(false);
    }
  }, []);
  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if (
        event.key.toLowerCase() !== "c" ||
        !(event.metaKey || event.ctrlKey) ||
        event.altKey ||
        event.shiftKey ||
        !current.current.selection.nodeId
      )
        return;
      if (
        target.closest(
          "input, textarea, select, [contenteditable], [role=textbox]",
        ) ||
        window.getSelection()?.toString()
      )
        return;
      event.preventDefault();
      void copy();
    };
    window.addEventListener("keydown", keydown);
    return () => window.removeEventListener("keydown", keydown);
  }, [copy]);
  useEffect(() => {
    setMenu(null);
  }, [selection.projectId, selection.pageId, selection.frameId]);
  useEffect(() => {
    if (!menu) return;
    menuRef.current?.querySelector("button")?.focus();
    const outside = (event: PointerEvent) => {
      if (!menuRef.current?.contains(event.target as Node)) setMenu(null);
    };
    const escape = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        setMenu(null);
        returnFocus.current?.focus();
      }
    };
    window.addEventListener("pointerdown", outside);
    window.addEventListener("keydown", escape);
    return () => {
      window.removeEventListener("pointerdown", outside);
      window.removeEventListener("keydown", escape);
    };
  }, [menu]);
  const open = useCallback((id: string, x: number, y: number) => {
    returnFocus.current = document.activeElement as HTMLElement;
    current.current.select(id);
    setMenu({
      id,
      x: Math.max(8, Math.min(x, innerWidth - 200)),
      y: Math.max(8, Math.min(y, innerHeight - 140)),
    });
  }, []);
  const actions = useMemo<Actions>(() => ({
    copy,
    image,
    open,
  }), [copy, image, open]);
  return (
    <Context.Provider value={actions}>
      <BusyContext.Provider value={busy}>{children}</BusyContext.Provider>
      {menu && (
        <div
          ref={menuRef}
          className="node-context-menu"
          role="menu"
          aria-label="Element actions"
          style={{ left: menu.x, top: menu.y }}
          onKeyDown={(event) => {
            const buttons = [
              ...menuRef.current!.querySelectorAll<HTMLButtonElement>(
                "button:not(:disabled)",
              ),
            ];
            const at = buttons.indexOf(
              document.activeElement as HTMLButtonElement,
            );
            if (["ArrowDown", "ArrowUp", "Home", "End"].includes(event.key)) {
              event.preventDefault();
              const index =
                event.key === "Home"
                  ? 0
                  : event.key === "End"
                    ? buttons.length - 1
                    : (at +
                        (event.key === "ArrowDown" ? 1 : -1) +
                        buttons.length) %
                      buttons.length;
              buttons[index]?.focus();
            }
            if (event.key === "Tab") setMenu(null);
          }}
        >
          <button role="menuitem" onClick={() => void copy(menu.id)}>
            Copy link <span>⌘/Ctrl+C</span>
          </button>
          <button
            role="menuitem"
            disabled={busy}
            onClick={() => void image("png", menu.id)}
          >
            Export PNG
          </button>
          <button
            role="menuitem"
            disabled={busy}
            onClick={() => void image("svg", menu.id)}
          >
            Export SVG
          </button>
        </div>
      )}
      {message && (
        <div className="node-action-notice" role="status">
          <span>{message}</span>
          <button
            aria-label="Dismiss action message"
            onClick={() => setMessage("")}
          >
            ×
          </button>
        </div>
      )}
    </Context.Provider>
  );
}
