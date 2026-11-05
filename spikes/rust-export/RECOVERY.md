# Task09 recovery — 2026-11-04

**Historical recovery snapshot.** Task09 was subsequently completed as a measured spike on 2026-11-04;
see [README Part E](README.md#phần-e--task09-wasm-trong-chromium-web-worker-2026-11-04) for the Chromium Worker
measurements and the performance **no-go on the heavily loaded host**. The blockers/untested status below describe
the recovery point, not the final spike result. Production still uses Chromium; Task10–13 were not implemented.

## Branch and merge

- Worktree: `task09-wasm-recovery` under the session-approved temporary directory.
- Task09 fetched from `origin/feat/task09-wasm-spike` at
  `c56096157a3d51d14bafdfe36caa7f9fbb04784b` (implementation: `935ffd0`).
- Merge direction: **`codex/compose-postgres` into Task09**, source commit
  `dbd37a05e774184a231c62eee9650ccc029d0d0d`.
- The new worktree was clean before merging. No push, deployment or application data access.
- Three conflicts: `crates/server/src/capabilities.rs` (static restriction text),
  `crates/server/skills/implement-frame.md` and `docs/agents/LOG.md`.
  Kept responsive support from Compose, ancestor-hover guidance from Task09's main
  ancestry, and both committed log tails. No conflicting runtime algorithm required a decision.
- Task09 spike source remains byte-identical to its fetched commit. Other runtime changes
  are inherited from the two merge parents, not new spike implementation.
- The original Compose checkout stays on its original commit with its uncommitted log intact.
  Its log SHA-256 before/after recovery checks was
  `881d5adfd4b254170701e5e0f6324968445943324ac2faa721959331f80091ec`.

## Recovered state and evidence limits

Source already includes a shared painter library, manual C-ABI WASM wrapper,
no-SIMD/SIMD build script (three optimization profiles), six scene/DPR cases,
native frame benchmark, Wasmtime comparison runner, and an **untested** Chromium
Web Worker runner. No implementation was added during recovery.

The authorized sandbox checkout `/home/admin/OpenCanvas` is clean at the same
`c560961` commit. Its Task09 log ends at worker assignment, like the fetched log.
Only a native release binary/build cache was found: no `out/wasm`, WASM build
directories, Task09 raw measurements or additional uncommitted source in the
checked spike paths. No files were copied back. Sandbox access was read-only,
using a separate known-hosts file (first-contact accept-new, then strict checking).

The README has no Task09/Part E results or rerun section, despite source comments
referring to one. `docs/PURE-RUST-EXPORT.md` and the task status still describe the
old deferred/unassigned state; they are not evidence that Task09 passed.

## Checks on the merged tree

| Check | Result |
| --- | --- |
| `cargo check --locked --manifest-path spikes/rust-export/Cargo.toml` | **Blocked:** E0308 twice at `src/convert.rs:28`; layout viewport became `f64`, spike viewport is `f32` |
| `cargo check --offline --locked --target wasm32-unknown-unknown --manifest-path spikes/rust-export/wasm/Cargo.toml` | Pass (type-check only, not release size/timing evidence) |
| `cargo test --offline --locked -p opencanvas-layout --lib responsive -- --test-threads=1` | 5 passed |
| `cargo fmt --all --check` | Pass (root workspace only) |
| `node --check` on both WASM runner files; `bash -n` on `wasm/build.sh` | Pass |
| Conflict-resolution file whitespace checks / unmerged index entries | Clean / none |

The initial offline native check lacked cached `crossbeam-deque 0.8.8`. A locked
online check downloaded normal project dependencies and exposed the type mismatch.
No toolchain targets or global tools were installed.

Whole-merge whitespace checking reports existing trailing whitespace inherited
from Compose's notice-bar commit in `web/src/App.tsx` and `web/src/components/Topbar.tsx`.
Those files were not manually edited. No server/storage/full-product suite or
benchmark was run. This is a recovered merge, **not a green Task09 acceptance**.

All spike source files inspected are below 200 lines (largest: 161).
Resolved capability/skill files are 123/187 lines. `AGENTS.md`: 69 lines, 9,476 bytes.

## Toolchains and next worker

- Host: macOS arm64, Rust/Cargo 1.99.1; `wasm32-unknown-unknown` already installed.
  Node 24.21.0, npm 11.19.0, Playwright 1.64.1, pinned Chromium 156.0.8078.4.
  A launch-and-close probe of the pinned headless browser passed; `brotli` exists.
- Host dependencies/browsers are in the original checkout, not linked into this worktree.
- Sandbox: Linux aarch64, Rust 1.99.1 with native and WASM targets, `brotli` present;
  no Node/npm on SSH PATH, no repo Playwright or pinned browser directory.

Remaining AC: release raw/gzip/brotli sizes for both variants; instantiate/cold/
20-warm p50/p95 for frame DPR1/2 and quarter tile; matching native timings;
native/WASM pixel differences and cause; README tables and a justified go/no-go
in `docs/PURE-RUST-EXPORT.md`. Wasmtime cannot substitute for the Chromium Worker
gate. The old sandbox log records permission to skip browser measurement there;
the host can now support it, but recovery did not run it.

First fix the viewport adapter **only inside the spike** and verify precision
semantics; then prepare the worktree's local dependencies/browser links and run:

```sh
# From spikes/rust-export, after fixing the adapter:
cargo build --release --locked --manifest-path Cargo.toml
target/release/rust-spike frames data/scene.json out/wasm/scenes
target/release/rust-spike frame-bench out/wasm/scenes --threads 1 --iters 20 \
  --dump out/wasm/native --json out/wasm/native.json
bash wasm/build.sh
node js/wasm-bench.mjs nosimd-release simd128-release
```

Before timing, review the untested runner's worker-error handling, failure exit
status, per-render return codes and cleanup when browser launch fails. Confirm
both RGBA comparisons and correct quarter-tile semantics. Keep generated outputs
under ignored `out/wasm/`; do not commit them or alter the live app/toolchain.
