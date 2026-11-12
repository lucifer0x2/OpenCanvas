# React/SWR local preview — 2026-11-12

The current dirty `codex/react-frontend-optimization` frontend is running at
**http://127.0.0.1:3877**. It serves a frozen production build and proxies all
`/api` requests, including SSE, to the existing native server on loopback 3857.
That native server continues to use the existing PostgreSQL database on 2703.
This is a frontend preview, not a native installation or a separate backend.

## Operation and provenance

The detached Vite preview process started as PID **71004**. Its private, ignored
runtime directory is `.runtime/react-swr-preview-3877/`:

- `server.pid`, `server.log`: current process marker and log.
- `workspace/web/dist/`: frozen production assets, independent of later web builds.
- `start.sh`, `launch.mjs`, `stop.sh`: temporary launch and guarded graceful stop.
- `SOURCE_HEAD`, `SOURCE_SHA256`, `SCENE_BUILD_FINGERPRINT`, `DIST_SHA256`:
  source revision, actual dirty source digests, scene-build identity and asset digests.
- `evidence/verification.json`, `browser-wasm.png`, `browser-server.png`:
  read-only verification and screenshots; not committed.

Base HEAD is `f54f7c2a841c246b27c70b783d9b6d28778a8b9f` with the accepted dirty
React/SWR changes. The fresh `npm --prefix web run build` passed TypeScript and
Vite: 221 modules; entry `index-DKZPw7Jh.js` is 246.37 kB, gzip 78.77 kB.
The build warns about SWR's `use client` directive; this client-only Vite app
rendered successfully in Chromium. All 18 frozen dist files have SHA-256 records.
The served HTML and production entry bytes were compared with the frozen files.

A targeted `cargo build --locked --offline -p opencanvas-server -p
opencanvas-layout` also passed while evaluating a separate-backend option.
Those copied binaries were **not launched**. No private database configuration
is loaded by the final preview launcher.

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
  byte-for-byte, at revisions **8, 10, 18, 60, 188, 342**. Per-project SHA-256
  evidence stays ignored and private. Revisions can advance during later use.
- JavaScript MIME, WASM MIME and WASM `0061736d` magic passed. A proxied project
  SSE stream returned `text/event-stream` and a durable `commit` replay.
- Actual Chromium rendered an existing revision-18 frame in both modes, with
  painted canvas pixels, **Live**, zero console/page errors and zero authored
  requests. Default mode fetched WASM once and made no server-scene request;
  explicit `?scene=server` made one server-scene request. Neither used a fallback.
  Both screenshots were captured; the default screenshot was visually inspected.
- An initial smoke link omitted the required `node` parameter and therefore
  selected the first project instead. That project rendered successfully at r10;
  the harness was corrected to use the complete canonical link before acceptance.

The backend's broadcast is process-local. A separate backend connected to the
same PostgreSQL database would replay durable commits when reopening its project
stream, but native MCP mutations would not wake that backend's existing streams;
workspace hints are also process-local. This preview avoids that limitation by
proxying SSE to the same authoritative native process that accepts mutations.
No new authored mutation was performed to test a future live event; the check
verified the actual proxy stream/replay and browser Live connection.

No native config, public listener, Funnel, project/auth settings, or database data
was changed. No deployment, restart, commit or push occurred. The preview's local
API proxy has the same local API capabilities as native 3857; keep it on loopback.
No handwritten runtime script exceeds 200 lines.
