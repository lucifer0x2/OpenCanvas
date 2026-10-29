# Whole-app build and launch — 2026-10-29

Root commands now build the renderer bundle, Rust layout/server binaries and compiled
viewer: `npm run build` selects debug; `npm run build:release` selects optimized Rust.
`npm run dev` and `npm start` launch the existing debug/release artifacts respectively.
The launchers serve `web/dist`, forward server arguments, and select the matching
layout worker through `OPENCANVAS_LAYOUT_BIN`. The startup fingerprint hashes that
same binary. Direct server/Docker starts keep their existing debug-layout default.

`scripts/app.mjs` owns process orchestration only. Cargo and npm retain their existing
build responsibilities. The launcher resolves its repository from its module URL,
supports explicit `CARGO_TARGET_DIR` (relative to that repository), forwards failures
and termination, and reports the matching build command for missing artifacts.
It installs neither Node dependencies nor Chromium. Both profiles share viewer and
renderer bundles.

## Verification

Executed with existing dependencies and Chromium; no installations/downloads:

```sh
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/private/tmp/opencanvas-p24-target npm run build -- --offline
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/private/tmp/opencanvas-p24-target npm run build:release -- --offline
CARGO_BUILD_JOBS=2 CARGO_TARGET_DIR=/private/tmp/opencanvas-p24-target cargo clippy --offline -p opencanvas-server --lib -- -D warnings
rustfmt --edition 2021 --check crates/server/src/fingerprint.rs
node --check scripts/app.mjs
node_modules/.bin/prettier --check scripts/app.mjs
git diff --check
```

All passed. The release build compiled both Rust executables with the optimized
profile. Viewer compilation includes its TypeScript check. Vite reports its existing
chunk-size warning (583.78 kB JavaScript); it does not fail the build.

Two isolated live smoke runs invoked `npm run dev` and `npm run start` with
`-- --port 0 --data-root <temporary-directory>` and the target directory above.
`PLAYWRIGHT_BROWSERS_PATH` selected the existing installed Chromium. Both passed:

- Viewer HTML and its compiled JavaScript asset returned HTTP 200; no Vite client.
- `docker/smoke.mjs` completed all seven checks: viewer, legacy MCP handshake/tools,
  2026 discovery/tools, create project, apply page/frame/text, render ready, PNG fetch.
- Text rendering used Chromium 156.0.8078.4; both PNGs were 320 × 200, 8,295 bytes.
- A deliberately invalid inherited layout override was replaced by the launcher's
  selected profile path; rendering still completed.
- SIGTERM sent specifically to the launcher reached the server as graceful SIGINT;
  the server printed `Stopping…` and the npm process exited 0.

The coordinator independently reran both npm launch commands with a temporary
target containing only that profile's two binaries and an invalid inherited layout
override. Both served compiled HTML/JavaScript and rendered a Vietnamese text
fixture to a 320 × 200 PNG through MCP. Temporary data and process groups were
cleaned after each run.

Temporary subprocess probes additionally checked both profiles from an external
working directory with a relative `CARGO_TARGET_DIR`: correct selected layout path,
preserved spaced CLI argument, child exit 17 propagated unchanged, and profile-specific
missing-artifact guidance. An invalid Cargo option made `npm run build` fail and stop
before the viewer build. Temporary target directories and live data were cleaned up; live
process groups were terminated. The initial sandboxed live attempt could not bind
loopback; the successful live checks used approved execution outside the sandbox.

No server on port 3847, production data, Docker image, or original source files were
changed. A temporary `web/node_modules` symlink reused installed dependencies for QA;
the coordinator removed that symlink after independent verification.

## Scope and limits

Cross-platform path/process handling is implemented with Node APIs; the actual run
was on macOS. Runtime sources/bundles must remain available; this is a repository
build workflow, not a standalone installer. Updating runtime artifacts still requires
restarting the backend so cache identity matches the active pipeline.

Changed handwritten code remains below 250 lines: `scripts/app.mjs` 94 lines,
`crates/server/src/fingerprint.rs` 56. Configuration: `package.json` 24 lines.
Documentation: `README.md` 102 lines and `docs/SERVER.md` 346 lines; the latter is
existing reference documentation, outside the handwritten-code threshold.
