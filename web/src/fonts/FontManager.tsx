import { useEffect, useRef, useState } from "react";
import { Icon } from "../components/Icon";
import { FontImportForm } from "./FontImportForm";
import { FontList } from "./FontList";
import { useFonts } from "./useFonts";
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
  const [open, setOpen] = useState(false);
  const fonts = useFonts(projectId, revision, open, changed);
  useEffect(() => {
    if (open && !dialog.current?.open) dialog.current?.showModal();
  }, [open]);
  const close = () => {
    dialog.current?.close();
    setOpen(false);
    trigger.current?.focus();
  };
  return (
    <>
      <button
        ref={trigger}
        aria-haspopup="dialog"
        disabled={!projectId || revision === undefined}
        onClick={() => setOpen(true)}
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
        onClose={() => setOpen(false)}
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
        <div className="fonts-body">
          <p id="fonts-description">
            Fonts travel with your project. Import files here without installing
            them on your computer. Bundled Noto Sans is also available to
            designs.
          </p>
          <section
            className="fonts-library"
            aria-labelledby="fonts-library-heading"
          >
            <div className="fonts-section-heading">
              <h3 id="fonts-library-heading">Available fonts</h3>
              <button
                type="button"
                onClick={fonts.refresh}
                disabled={fonts.loading}
              >
                Refresh list
              </button>
            </div>
            {fonts.loading && (
              <p role="status" className="subtle">
                Loading fonts…
              </p>
            )}
            {fonts.data && (
              <FontList fonts={fonts.data.fonts} download={fonts.download} />
            )}
          </section>
          {fonts.error && (
            <p className="font-error font-feedback" role="alert">
              {fonts.error}
            </p>
          )}
          {fonts.notice && (
            <p className="font-notice font-feedback" role="status">
              {fonts.notice}
            </p>
          )}
          <FontImportForm
            busy={fonts.busy}
            ready={!!fonts.data && !fonts.loading}
            retryable={fonts.retryable}
            submit={fonts.submit}
          />
        </div>
      </dialog>
    </>
  );
}
