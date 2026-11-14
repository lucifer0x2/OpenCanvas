import { useViewerAccess } from "../hooks/useViewerAccess";
import { FontImportForm } from "./FontImportForm";
import { FontList } from "./FontList";
import { useFonts } from "./useFonts";

interface Props {
  projectId: string;
  revision?: number;
  open: boolean;
  changed: () => void;
}

/** Mounted on first opening, retained while closed so recoverable form work survives. */
export function FontManagerContent({
  projectId,
  revision,
  open,
  changed,
}: Props) {
  const fonts = useFonts(projectId, revision, open, changed);
  const readOnly = useViewerAccess() !== false;
  return (
    <div className="fonts-body">
      <p id="fonts-description">
        {readOnly
          ? "Public viewer: browse and download the fonts used by this project."
          : "Fonts travel with your project. Import files here without installing them on your computer. Bundled Noto Sans is also available to designs."}
      </p>
      <section className="fonts-library" aria-labelledby="fonts-library-heading">
        <div className="fonts-section-heading">
          <h3 id="fonts-library-heading">Available fonts</h3>
          <button type="button" onClick={fonts.refresh} disabled={fonts.loading}>
            Refresh list
          </button>
        </div>
        {fonts.loading && (
          <p role="status" className="subtle">Loading fonts…</p>
        )}
        {fonts.data && (
          <FontList fonts={fonts.data.fonts} download={fonts.download} />
        )}
      </section>
      {fonts.error && (
        <p className="font-error font-feedback" role="alert">{fonts.error}</p>
      )}
      {fonts.notice && (
        <p className="font-notice font-feedback" role="status">{fonts.notice}</p>
      )}
      {!readOnly && (
        <FontImportForm
          busy={fonts.busy}
          ready={!!fonts.data && !fonts.loading}
          retryable={fonts.retryable}
          submit={fonts.submit}
        />
      )}
    </div>
  );
}
