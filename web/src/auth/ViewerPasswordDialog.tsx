import { useEffect, useRef, useState, type FormEvent } from "react";
import { unlockViewer, ViewerLoginError } from "./api";

export function ViewerPasswordDialog({
  onUnlock,
}: {
  onUnlock: (required: boolean) => void;
}) {
  const dialog = useRef<HTMLDialogElement>(null);
  const input = useRef<HTMLInputElement>(null);
  const button = useRef<HTMLButtonElement>(null);
  const pending = useRef<AbortController | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<ViewerLoginError>();

  useEffect(() => {
    const element = dialog.current;
    element?.showModal();
    input.current?.focus();
    return () => {
      pending.current?.abort();
      element?.close();
    };
  }, []);

  const submit = async (event: FormEvent<HTMLFormElement>) => {
    event.preventDefault();
    const field = input.current;
    if (pending.current || !field?.value) return;
    const controller = new AbortController();
    pending.current = controller;
    setBusy(true);
    setError(undefined);
    const request = unlockViewer(field.value, controller.signal);
    field.value = "";
    try {
      const required = await request;
      if (!controller.signal.aborted) onUnlock(required);
    } catch (cause) {
      if (controller.signal.aborted) return;
      setError(
        cause instanceof ViewerLoginError
          ? cause
          : new ViewerLoginError("Could not unlock the viewer. Try again."),
      );
      field.focus();
    } finally {
      if (!controller.signal.aborted) {
        pending.current = null;
        setBusy(false);
      }
    }
  };

  return (
    <dialog
      ref={dialog}
      className="viewer-password-dialog"
      aria-labelledby="viewer-password-title"
      aria-describedby="viewer-password-help"
      onCancel={(event) => event.preventDefault()}
      onKeyDown={(event) => {
        if (event.key !== "Tab") return;
        const first = input.current;
        const last = button.current;
        if (event.shiftKey && document.activeElement === first) {
          event.preventDefault();
          last?.focus();
        } else if (!event.shiftKey && document.activeElement === last) {
          event.preventDefault();
          first?.focus();
        }
      }}
    >
      <p className="viewer-password-brand">OpenCanvas</p>
      <h1 id="viewer-password-title">Unlock the viewer</h1>
      <p id="viewer-password-help">
        Enter the shared password to view the designs.
      </p>
      <form onSubmit={submit} aria-busy={busy}>
        <label htmlFor="viewer-password">Password</label>
        <input
          ref={input}
          id="viewer-password"
          name="password"
          type="password"
          autoComplete="current-password"
          required
          readOnly={busy}
          aria-invalid={error?.incorrectPassword}
          aria-describedby={error ? "viewer-password-error" : undefined}
          onChange={() => setError(undefined)}
        />
        {error && (
          <p id="viewer-password-error" role="alert">
            {error.message}
          </p>
        )}
        <button ref={button} type="submit" aria-disabled={busy}>
          {busy ? "Unlocking…" : "Unlock viewer"}
        </button>
      </form>
    </dialog>
  );
}
