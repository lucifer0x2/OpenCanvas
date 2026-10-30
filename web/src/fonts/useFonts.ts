import { useEffect, useRef, useState } from "react";
import { FontImportError, downloadFont, importFont, listFonts } from "./api";
import { readFontFile } from "./fontFiles";
import type { FontDetails, FontImport, FontList, ProjectFont } from "./types";

/** One project per mounted manager; requests and immutable retries never cross projects. */
export function useFonts(
  projectId: string,
  revision: number | undefined,
  open: boolean,
  changed: () => void,
) {
  const [data, setData] = useState<FontList | null>(null);
  const [loading, setLoading] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState("");
  const [loadError, setLoadError] = useState("");
  const [notice, setNotice] = useState("");
  const [retryable, setRetryable] = useState(false);
  const [reload, setReload] = useState(0);
  const frozen = useRef<FontImport | null>(null);
  const lifetime = useRef(new AbortController());
  const refresh = () => setReload((value) => value + 1);
  useEffect(() => {
    const controller = new AbortController();
    lifetime.current = controller;
    return () => controller.abort();
  }, []);
  useEffect(() => {
    if (!open || revision === undefined) return;
    const controller = new AbortController();
    setLoading(true);
    listFonts(projectId, controller.signal)
      .then((next) => {
        if (!controller.signal.aborted) {
          setData(next);
          setLoadError("");
        }
      })
      .catch((cause) => {
        if (!controller.signal.aborted) {
          setData(null);
          setLoadError(cause.message ?? "Could not load project fonts.");
        }
      })
      .finally(() => {
        if (!controller.signal.aborted) setLoading(false);
      });
    return () => controller.abort();
  }, [projectId, revision, open, reload]);

  const submit = async (file?: File, details?: FontDetails) => {
    if (busy || (!frozen.current && (!data || !file || !details))) return false;
    const signal = lifetime.current.signal;
    setBusy(true);
    setError("");
    setNotice("");
    let sent = false;
    try {
      if (!frozen.current && file && details && data) {
        const bytes = await readFontFile(file);
        if (signal.aborted) return false;
        const identity = crypto.randomUUID();
        frozen.current = {
          expectedRevision: data.revision,
          idempotencyKey: `font-import-${identity}`,
          ...bytes,
          font: {
            id: `font_${identity}`,
            family: details.family.trim(),
            weight: details.weight,
            style: details.style,
            ...(details.license.trim()
              ? { license: details.license.trim() }
              : {}),
          },
        };
      }
      sent = true;
      await importFont(projectId, frozen.current!, signal);
      if (signal.aborted) return false;
      const family = frozen.current!.font.family;
      frozen.current = null;
      setRetryable(false);
      setNotice(
        `${family} was added to this project. No design fonts were replaced.`,
      );
      refresh();
      changed();
      return true;
    } catch (cause) {
      if (signal.aborted) return false;
      const failure = cause as Error;
      const ambiguous =
        sent && (!(failure instanceof FontImportError) || failure.ambiguous);
      setRetryable(ambiguous);
      if (ambiguous)
        setError(
          "The import response was interrupted. Retry the same import to safely check its result.",
        );
      else {
        frozen.current = null;
        if (
          failure instanceof FontImportError &&
          failure.code === "REVISION_CONFLICT"
        ) {
          setError(
            "This project changed before the import. Refreshing its fonts; review the list and submit Add font again.",
          );
          refresh();
          changed();
        } else setError(failure.message || "The font could not be added.");
      }
      return false;
    } finally {
      if (!signal.aborted) setBusy(false);
    }
  };
  const download = async (font: ProjectFont) => {
    setError("");
    try {
      await downloadFont(projectId, font, lifetime.current.signal);
      if (!lifetime.current.signal.aborted)
        setNotice(`Downloaded ${font.family} (${font.weight} ${font.style}).`);
    } catch (cause) {
      if (!lifetime.current.signal.aborted) setError((cause as Error).message);
    }
  };
  return {
    data,
    loading,
    busy,
    error: error || loadError,
    notice,
    retryable,
    refresh,
    submit,
    download,
  };
}
