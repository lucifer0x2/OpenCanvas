# React/SWR local preview — 2026-11-12

The current dirty `codex/react-frontend-optimization` frontend, including URL-owned
project/page/frame selection and optional node selection, is running at
**http://127.0.0.1:3877**. It serves a frozen production build and proxies all
`/api` requests, including SSE, to the existing native server on loopback 3857.
That native server continues to use the existing PostgreSQL database on 2703.
This is a frontend preview, not a native installation or a separate backend.

## Operation and provenance

The refreshed detached Vite preview process started as PID **69276**. Its private, ignored
runtime directory is `.runtime/react-swr-preview-3877/`:

- `server.pid`, `server.log`: current process marker and log.
- `workspace/web/dist/`: frozen production assets, independent of later web builds.
- `start.sh`, `launch.mjs`, `stop.sh`: temporary launch and guarded graceful stop.
- `SOURCE_HEAD`, `SOURCE_SHA256`, `SCENE_BUILD_FINGERPRINT`, `DIST_SHA256`:
  source revision, actual dirty source digests, scene-build identity and asset digests.
- `evidence/verification.json`, `browser-wasm.png`, `browser-server.png`:
  read-only verification and screenshots; not committed.
- `backups/pre-url-20261112/`: previous frozen candidate, scripts, process marker,
  source/asset manifests and evidence, preserved for rollback.

Base HEAD is React/SWR checkpoint `72a449901c36d34a101b0f8fd5ad4dd7687e5464`,
plus the accepted uncommitted URL-navigation changes described in
[URL-NAVIGATION.md](URL-NAVIGATION.md). This refresh reused the final production
build without rebuilding. Entry `index-D-g5kYFV.js` has SHA-256
`243ad180db4c68d2f319c82820bff636c9a8e194d4a61dbf0d592dadf18aa582`;
canvas chunk `App-BmQEvNlc.js` has SHA-256
`396049f09c64638e02d5cc6a28ee1ad63758ee41d6c8bbb0b4a3440102ced372`.
The frozen source and current source produce the same scene-build fingerprint:
`a40d6a6644fd0088285d539851de19b5faead4b03b174430c78b5e72797bf9a0`.
All 18 frozen dist files match both the repository build and raw served HTTP
bytes, including compressed assets and WASM. Source digests include ignored
generated WASM bytes. Copied legacy Rust binaries remain **unlaunched**; the
launcher loads no private database configuration.

From the repository root, stop only this preview:

```sh
bash .runtime/react-swr-preview-3877/stop.sh
```

To start the same frozen frontend after stopping it:

```sh
bash .runtime/react-swr-preview-3877/start.sh
```

The scripts refuse an active PID or a mismatched stop target. Port 3877 is strict;
they do not pick another port or replace another process. The preview depends on
the existing checkout's Node/Vite dependencies and native API. It is temporary,
survives the tool turn, and has no launchd/autostart configuration.

To roll back, stop with the guarded script, move the current `workspace/` aside,
restore `backups/pre-url-20261112/workspace/` to `workspace/`, restore the four
source/asset manifests from that backup, and run `start.sh`. Keep the active
`server.pid` marker; do not restore the historical PID. The backup remains intact.

Rerun the read-only checks using the installed Chromium:

```sh
PLAYWRIGHT_BROWSERS_PATH="$PWD/.runtime/browsers" \
  node .runtime/react-swr-preview-3877/verify.mjs
```

## Verification and boundaries

- Preview binds only `127.0.0.1:3877`. Native PID 2652 remains on 3857/3858,
  PostgreSQL PID 2811 remains on 2703, and unrelated PID 17418 remains on 3867.
  The existing process identities/start times were checked without restarting them.
- Native PID 2652 has four established TCP connections to loopback 2703.
  Their client ports matched read-only `pg_stat_activity` rows for the app role.
  No connection string or credential was printed or stored in the report.
- Project lists and all **6 raw authored snapshot responses** matched native
  byte-for-byte, at revisions **8, 10, 18, 60, 193, 343**. Per-project SHA-256
  evidence stays ignored and private. Revisions can advance during later use.
- JavaScript MIME, WASM MIME and WASM `0061736d` magic passed. A proxied project
  SSE stream returned `text/event-stream` and a durable `commit` replay.
- Actual Chromium rendered an existing revision-18 frame in both modes, with
  painted canvas pixels, **Live**, zero console/page errors and zero authored
  requests. Across navigation and two reloads, default mode fetched WASM three
  times and made no server-scene request; explicit `?scene=server` made five
  server-scene requests and fetched no WASM. Neither used a fallback.
  Both screenshots were captured; the default screenshot was visually inspected.
- Three-key links (`project`, `page`, `frame`) open the intended scope with no
  `node` parameter and an empty Inspector. Layer and canvas clicks add `node`;
  canvas-background deselection removes it. Selected and deselected states
  survive reload. Back/Forward restores node and frame/All scope. Unrelated
  query parameters, `scene=server`, the hash and another owner's `history.state`
  survive these operations in both modes. The old preview's required-node
  workaround is no longer needed.

The backend's broadcast is process-local. A separate backend connected to the
same PostgreSQL database would replay durable commits when reopening its project
stream, but native MCP mutations would not wake that backend's existing streams;
workspace hints are also process-local. This preview avoids that limitation by
proxying SSE to the same authoritative native process that accepts mutations.
No new authored mutation was performed to test a future live event; the check
verified the actual proxy stream/replay and browser Live connection.

No native config, public listener, Funnel, project/auth settings, or database data
was changed. Only the owned temporary frontend was restarted; no native
deployment/restart, commit or push occurred during this refresh. The preview's local
API proxy has the same local API capabilities as native 3857; keep it on loopback.
No handwritten runtime script exceeds 200 lines.
