import { Component, Suspense, type ReactNode } from "react";

/** Chunk loading stays behind ViewerGate; retry reloads failed cached module imports. */
export class RouteBoundary extends Component<
  { children: ReactNode },
  { failed: boolean }
> {
  state = { failed: false };

  static getDerivedStateFromError() {
    return { failed: true };
  }

  render() {
    if (this.state.failed)
      return (
        <main className="viewer-access-status">
          <p role="alert">Could not load this viewer.</p>
          <button type="button" onClick={() => window.location.reload()}>
            Try again
          </button>
        </main>
      );
    return (
      <Suspense
        fallback={
          <main className="viewer-access-status">
            <p role="status">Loading viewer…</p>
          </main>
        }
      >
        {this.props.children}
      </Suspense>
    );
  }
}
