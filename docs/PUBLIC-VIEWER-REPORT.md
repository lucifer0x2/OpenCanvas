# Public read-only viewer — 2026-10-31

Implementation and isolated verification are complete. Public sharing remains
opt-in: the existing local listener keeps MCP/project creation/font import,
while a separate listener shares the viewer, live updates, resources, downloads
and computed exports. Runtime rollout is owned by the root orchestrator and is
recorded separately below; the implementation worker did not restart port 3847,
run Docker services, enable Funnel, or access production project data.

## Design and changed modules

- `crates/server/src/public_viewer.rs`: validated HTTPS-origin configuration,
  independent public router and method/Host/Origin gate. The route graph contains
  no MCP, project-creation or font-import handler. Forwarded headers are ignored
  for privilege decisions. GET/HEAD and only the exact POST exports path are
  accepted; encoded paths, repeated slashes, dot segments, MCP variants and all
  other mutation methods fail closed. Host/Origin checks never grant writes.
- `crates/server/src/http.rs`: shared read/export routes, local-only write/MCP
  registration, and listener capability endpoint. Both route graphs retain the
  15 MiB body budget. `main.rs` adds paired public CLI flags, distinct port
  validation, random ports and shared graceful shutdown including SSE.
- `crates/server/src/viewer_origin.rs`, `fonts.rs`, `export/http.rs`: project only
  generated scene resource, image, export-file and font-download links onto the
  configured public origin. Cached jobs, worker callbacks and local responses
  retain loopback links; authored strings are never globally rewritten.
- `web/src/fonts/useViewerAccess.ts` and `FontManager.tsx`: public Fonts dialog
  retains inventory/downloads and hides import; local import remains available.
  Server enforcement is independent of this UI capability.
- `compose.public-viewer.yaml`: optional override, retaining the local mapping
  and adding a second loopback-published port. Base Compose remains local-only.
- `README.md`, `docs/SERVER.md`, `CONTRACT.md`, `FONTS.md`: native/Docker/Funnel
  commands and explicit disclosure that all projects/resources become readable.
- Tests: `public_viewer.rs`, `public_resources.rs`, `public_cli.rs`, focused shared
  `public_support`, and `web/tests/public-viewer-{server,smoke}.mjs`.

## Verification

All server tests use isolated temporary roots and random loopback ports. The
browser check uses a temporary self-signed HTTPS proxy preserving Host/Origin
and streaming SSE, the production compiled viewer, actual native backend,
local MCP and Chromium. It cleans the proxy, backend, rendering processes,
certificate/private key and project root afterward.

| Command/check | Result |
| --- | --- |
| `cargo check -p opencanvas-server --offline` | Pass |
| `cargo test -p opencanvas-server --offline --test public_viewer --test public_resources -- --test-threads=1` | 4/4 pass |
| `cargo test -p opencanvas-server --offline --test public_cli -- --test-threads=1` | 2/2 pass; flags reject before opening data root; both listeners with active SSE stop on one SIGINT before the 3-second hard-exit fallback |
| Root independent server run: public_viewer, public_resources, transport, font_portability, viewer_exports, workspace_events, plus public_cli | 16/16 pass across seven focused suites |
| Root strict Clippy for server lib/binary with `-D warnings` | Pass |
| `cargo fmt --all --check` and `rtk bypass git diff --check` | Pass |
| `cargo build -p opencanvas-server --offline` | Pass |
| `npm --prefix web run typecheck` and `npm --prefix web test` | Pass; 25/25 unit tests |
| `npm --prefix web run build` | Pass; existing large-chunk advisory remains |
| `node --check web/tests/public-viewer-server.mjs` and `node --check web/tests/public-viewer-smoke.mjs` | Pass |
| `node web/tests/public-viewer-smoke.mjs` | Pass, rerun after final formatting; root independently repeated it successfully |
| `rtk bypass npx prettier --check web/src/fonts/FontManager.tsx web/src/fonts/useViewerAccess.ts web/tests/public-viewer-server.mjs web/tests/public-viewer-smoke.mjs compose.public-viewer.yaml` | Pass |
| `docker compose -f compose.yaml config --format json` | Default has only local 3847 mapping and no public flags |
| `OPENCANVAS_PUBLIC_ORIGIN=https://canvas.example.ts.net docker compose -f compose.yaml -f compose.public-viewer.yaml config --format json` | Both 3847/3848 mappings remain on 127.0.0.1; public flags/origin present |
| Override config with local/public ports 3850/3851 | Command and both loopback mappings follow overrides |
| Override config without OPENCANVAS_PUBLIC_ORIGIN | Expected exit 1 before starting any service |

The public gate test covers POST/PUT/PATCH/DELETE/OPTIONS across 19 denied path
variants with public, localhost and loopback Hosts plus spoofed forwarding
headers and matching local/public Origins. It also checks GET MCP variants,
wrong public Host/Origin, absent Origin navigation, unchanged revision/project
count, local Host protection, local HTTP creation and local MCP creation.

The resource integration imports a real font locally and an image via MCP,
then verifies public font/asset bytes, scene font/image URLs, ready PNG, actual
PNG/SVG exports with embedded font/image data, downloads and unchanged authored
revision. Public projection leaves the shared cached job and worker links local.
Project/workspace SSE receive actual local commits and end on shared shutdown.

The HTTPS browser run verifies viewer scene rendering, download-only public
Fonts, exact original-font bytes, SVG export through the real UI, a live MCP
update from revision 3 to 4, preview text/image/font resources, and local import
UI. Every public browser request stays on its configured HTTPS origin; its only
POST is exports. There are no page errors or failed resource requests. Screenshots
`viewer.png` and `preview.png` are retained under the OS temporary directory's
`opencanvas-public-evidence/`; both were visually inspected.

## File size and limits

All changed handwritten code is at most 247 lines. Largest files: browser smoke
247, HTTP routing 219, resource integration 196, public gate tests 193, font
handlers 138, Fonts dialog 135 and CLI main 134. Each new module has a focused
responsibility. No changed code file exceeds 250 lines. `docs/SERVER.md` is 454
lines; it is existing reference documentation rather than handwritten code.

Public sharing has no authentication, per-project filter or separate render
capacity. Anyone reaching it can read all projects, notes, uploaded fonts and
assets and request exports within the existing budgets. Use a separate root
to share a subset. The native HTTPS test models proxy behavior; it does not
claim external internet reachability or Tailscale DNS/TLS/policy validation.
No Docker image/container runtime was built or restarted by this worker.

## Runtime rollout

- Root built the release server with `cargo build --release -p opencanvas-server
  --offline` (pass) and verified the supplied public domain matches this machine's
  Tailscale DNS name: `https://macbook-air.tail486386.ts.net`.
- The existing native release server was gracefully restarted with its existing
  workspace/default data root and the additional flags `--public-viewer-port
  3848 --public-viewer-origin https://macbook-air.tail486386.ts.net`. A detached
  shell launch was reaped by the execution harness; the server was immediately
  restored in a managed foreground session. Both listeners are running.
- Live HTTP probes passed: local viewer capability is writable locally, public
  capability is read-only, and six POST probes across MCP/create/font-import with
  public or spoofed localhost Host headers all return `PUBLIC_READ_ONLY` (403).
  No project was created or changed by deployment/probes. Native runtime currently
  differs from the older Docker note in AGENTS; that note has been corrected.
- Automatic approval review rejected `tailscale funnel --bg
  http://127.0.0.1:3848` before execution: it requires explicit user approval for
  exposing **all** projects and their resources to the public internet at that
  domain. Funnel was not enabled; external reachability is not yet verified.
  The remaining action is user approval for this concrete publication scope,
  followed by the Funnel command and read-only HTTPS verification.

User subsequently requested stopping the server to run it personally. Root sent
SIGINT to the verified native server and confirmed both 3847 and 3848 have no
listener. Funnel was never enabled by this session. Do not restart or publish
automatically; the user now owns startup and sharing.
