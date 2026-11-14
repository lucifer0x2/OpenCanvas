import { useEffect, useRef, useState } from "react";
import { Icon } from "../components/Icon";
import { FontManagerContent } from "./FontManagerContent";
import "./fonts.css";

interface Props {
  projectId: string;
  projectName: string;
  revision?: number;
  changed: () => void;
}

/** Native modal makes the background inert; Tab wraps and Escape restores focus. */
export function FontManager({
  projectId,
  projectName,
  revision,
  changed,
}: Props) {
  const dialog = useRef<HTMLDialogElement>(null);
  const trigger = useRef<HTMLButtonElement>(null);
  const [visibility, setVisibility] = useState<"unopened" | "open" | "closed">(
    "unopened",
  );
  const open = visibility === "open";
  useEffect(() => {
    if (open && !dialog.current?.open) dialog.current?.showModal();
  }, [open]);
  const close = () => {
    dialog.current?.close();
    setVisibility("closed");
    trigger.current?.focus();
  };
  return (
    <>
      <button
        ref={trigger}
        aria-haspopup="dialog"
        disabled={!projectId || revision === undefined}
        onClick={() => setVisibility("open")}
      >
        <Icon name="text" />
        Fonts
      </button>
      <dialog
        ref={dialog}
        className="fonts-dialog"
        aria-labelledby="fonts-heading"
        aria-describedby="fonts-description"
        onCancel={(event) => {
          event.preventDefault();
          close();
        }}
        onClose={() => setVisibility("closed")}
        onKeyDown={(event) => {
          if (event.key !== "Tab") return;
          const controls = Array.from(
            event.currentTarget.querySelectorAll<HTMLElement>(
              'button:not(:disabled), input:not(:disabled), a[href], [tabindex="0"]',
            ),
          ).filter((element) => element.getClientRects().length > 0);
          const first = controls[0];
          const last = controls.at(-1);
          if (event.shiftKey && document.activeElement === first) {
            event.preventDefault();
            last?.focus();
          } else if (!event.shiftKey && document.activeElement === last) {
            event.preventDefault();
            first?.focus();
          }
        }}
      >
        <header className="fonts-header">
          <div>
            <h2 id="fonts-heading">Project fonts</h2>
            <p className="subtle">{projectName}</p>
          </div>
          <button type="button" aria-label="Close fonts" onClick={close}>
            Close
          </button>
        </header>
        {visibility !== "unopened" && (
          <FontManagerContent
            projectId={projectId}
            revision={revision}
            open={open}
            changed={changed}
          />
        )}
      </dialog>
    </>
  );
}
