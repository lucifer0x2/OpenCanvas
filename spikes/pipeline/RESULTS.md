# M0 pipeline verification — 2026-10-28

Workspace: OpenCanvas. Tests use in-memory fixture JSON and output under this spike; no `~/.opencanvas` data was created. No Git repository/commit existed when work began.

| Command actually run | Result |
| --- | --- |
| `cargo test -p opencanvas-layout` | 3 unit tests passed; missing measurements pending, missing font error, cycle/nonfinite geometry rejection |
| `npm run check:renderer` | TypeScript check passed |
| `npm run build:renderer` | Shared ESM bundle built; final bundle 11.2 KiB |
| `npx prettier --check 'renderer/*.ts' 'worker/*.mjs' 'spikes/pipeline/*.mjs'` | All files formatted (CLI fixture later individually formatted) |
| `cargo fmt -p opencanvas-layout` | Completed; Rust files conventionally formatted |
| `npm run test:pipeline` | Final integrated run: 8 passed, 0 failed/skipped, 1967.95ms; required loopback sandbox escalation |
| `node --test --test-concurrency=1 spikes/pipeline/lifecycle.test.mjs` | Targeted cancellation fix rerun: 2 passed, 0 failed |
| `node --test --test-concurrency=1 spikes/pipeline/cli.test.mjs` | Added executable protocol fixture: 1 passed, 0 failed, 336.99ms |

The current glob-based `npm run test:pipeline` discovers all nine fixtures. Eight were run together after the final runtime fix; the subsequently added CLI fixture passed separately and changed no implementation code. No full backend/viewer/SQLite E2E claim is made here.

Verified on Node24.21.0, Playwright1.64.1, Chromium156.0.8078.4, Taffy0.15.1, macOS arm64. Browser runtime installed inside `.runtime/browsers` and excluded from source.

Assertions proved:

- Text width380 produced two lines / height64; width200 produced four lines / height128. Nested row body fill/hug contributes its real measured height to group hug height.
- Wide first compute: 2 layout passes, 1 batched RPC, 3 measurements. Narrow: 2 passes, 1 batch, 2 measurements. Repeated wide: 1 pass, 0 RPC/measurements.
- Separate viewer/export canvases produce identical PNG bytes with `measureText` forced to throw during drawing; both use the same source scene key and manifests. Visually inspected actual generated PNG; Vietnamese diacritics, text lines, rounded rectangle and decoded image render correctly.
- Strict jobs reject missing font, wrong font hash and missing image. Non-strict viewer produces labeled image placeholder plus failed readiness/scene diagnostics; strict export of that scene fails.
- Real image `decode()` delayed180ms is awaited before compute readiness. An unresolved `decode()` hits the per-resource timeout and returns `RESOURCE_TIMEOUT`.
- Measurement non-convergence after a requested one-pass limit and render scale beyond budget produce structured failure.
- RGBA samples prove sibling paint order, frame-local nested x/y, ancestor clipping and normalized image source crop.
- A concurrent second job receives `WORKER_BUSY`; `cancel()` produces `JOB_CANCELLED`; worker refuses reuse until restarted.
- Executable `worker/cli.mjs` survives malformed JSON, then returns matching scene/strict PNG provenance over NDJSON, without exposing its local workspace path in the bad-request envelope.

Observed failures during development were corrected, not recorded as passes: sandbox `listen EPERM` (rerun with escalation); new Taffy callback signature compilation mismatch (adapted using real `compute_leaf_layout`); cancellation-close race (await page closure before browser shutdown).

Module review: handwritten source/test files max230 lines, no >250 exception. Largest responsibility files: tests230, worker226, DTO225, engine183, drawing166, resources152. Generated bundle/lockfiles/font/license are excluded from implementation line limits.

Known limits: missing-font computation fails explicitly (no fallback configuration in M0); isolated group alpha compositing, transforms, rich text and general CSS are unsupported. Cache is process-local, root frame fixed viewport, crop normalized, resource URLs controlled by backend, browser worker one job at a time. Runtime/process cancellation requires worker restart. These are documented in README and do not block the bounded M0 fixtures.
