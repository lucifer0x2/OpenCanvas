import { createContext, useContext, useEffect, useRef, useState } from "react";
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
  busy: boolean;
}
const Context = createContext<Actions | null>(null);
export const useNodeActions = () => useContext(Context);

export function NodeActionsProvider({
  children,
  selection,
  revision,
  select,
}: {
  children: ReactNode;
  selection: Selection;
  revision?: number;
  select: (id: string) => void;
}) {
  const [menu, setMenu] = useState<Position | null>(null);
  const [message, setMessage] = useState("");
  const [busy, setBusy] = useState(false);
  const menuRef = useRef<HTMLDivElement>(null);
  const returnFocus = useRef<HTMLElement | null>(null);
  const active = useRef(false);
  const copy = async (id = selection.nodeId) => {
    setMenu(null);
    try {
      await navigator.clipboard.writeText(
        nodeLink(location.href, { ...selection, nodeId: id }),
      );
      setMessage(
        "Link copied. Paste it into your AI chat to identify this element.",
      );
    } catch {
      setMessage(
        "Could not copy the link. Allow clipboard access and try again.",
      );
    }
  };
  const image = async (format: "png" | "svg", id = selection.nodeId) => {
    setMenu(null);
    if (active.current || revision === undefined) return;
    active.current = true;
    setBusy(true);
    setMessage(`Exporting ${format.toUpperCase()}…`);
    try {
      await exportNode(selection.projectId, id, revision, format);
      setMessage(`${format.toUpperCase()} downloaded.`);
    } catch (error) {
      setMessage(error instanceof Error ? error.message : "Export failed.");
    } finally {
      active.current = false;
      setBusy(false);
    }
  };
  useEffect(() => {
    const keydown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement;
      if (
        event.key.toLowerCase() !== "c" ||
        !(event.metaKey || event.ctrlKey) ||
        event.altKey ||
        event.shiftKey ||
        !selection.nodeId
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
  });
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
  const actions: Actions = {
    copy,
    image,
    busy,
    open: (id, x, y) => {
      returnFocus.current = document.activeElement as HTMLElement;
      select(id);
      setMenu({
        id,
        x: Math.max(8, Math.min(x, innerWidth - 200)),
        y: Math.max(8, Math.min(y, innerHeight - 140)),
      });
    },
  };
  return (
    <Context.Provider value={actions}>
      {children}
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
