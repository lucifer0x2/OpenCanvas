import type { ProjectFont } from "./types";

export function FontList({
  fonts,
  download,
}: {
  fonts: ProjectFont[];
  download: (font: ProjectFont) => void;
}) {
  return (
    <ul className="font-list" aria-label="Project fonts">
      {fonts.map((font) => (
        <li key={font.id}>
          <div className="font-info">
            <strong>{font.family}</strong>
            <span className="subtle">
              {font.weight} · {font.style} ·{" "}
              {font.source === "bundled"
                ? "Bundled with OpenCanvas"
                : "Stored in project"}
            </span>
            {font.license && (
              <span className="font-license subtle">
                License: {font.license}
              </span>
            )}
            {font.status !== "ready" && (
              <span className="font-error">
                Font file unavailable · {font.status}
              </span>
            )}
          </div>
          <button
            type="button"
            disabled={!font.downloadUrl || font.status !== "ready"}
            aria-label={`Download ${font.family}, ${font.weight} ${font.style}`}
            onClick={() => download(font)}
          >
            Download
          </button>
        </li>
      ))}
      {!fonts.length && <li>No fonts registered in this project.</li>}
    </ul>
  );
}
