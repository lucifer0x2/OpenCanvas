# Font import/export and content identity — 2026-10-30

## Scope and findings

Existing `import_asset` already stores original project font bytes under SHA-256
and registers a `Font` record. Existing project bundles already preserve uploaded
and pinned bundled font bytes. Fonts were not loaded by OS file paths, but the
browser registered them under authored family/weight/style and removed earlier
faces with that metadata. Different actual files could collide. Production SVG
previously referenced the readable family plus `sans-serif` without embedding
font bytes, requiring the recipient's installed fonts.

The product's pinned Noto Sans TTF and OFL already came from Google Fonts
(`renderer/fonts/manifest.json`). No production font replacement/download was
necessary. Added one original, unmodified official Google Fonts Lato TTF+OFL as
test data; provenance, pinned commit and hash are in `fixtures/fonts/README.md`.
It is not imported into any user project or used as a runtime fallback.

## Implementation

- Storage `read_font` resolves project assets or compiled pinned bundled bytes,
  verifies asset length/hash and the font manifest hash/status, and errors on
  missing/unknown/corrupt resources. No client paths or remote fetch.
- HTTP GET inventory, POST import and original-file attachment download. POST
  forwards through the existing domain import transaction, revision conflict,
  idempotency and events. List is bounded to 8 MiB. MCP `list_fonts` adds download
  metadata within its existing page byte/item budgets; tool catalog unchanged.
- Shared private CSS family `oc_<sha256>_<weight>_<style>` in FontFace loading,
  measurement, canvas and preview SVG text. Authored/readable `Font.family`
  remains unchanged. Renderer/layout version coordinated to `web-3`; pipeline
  fingerprint already hashes the renderer bundle and layout executable.
- Production SVG embeds exact store-owned font bytes in data `@font-face`
  rules, checks the computed scene hash/weight/style against current records,
  and never fetches scene-provided URLs. Font CSS is capped at 40 MiB inside the
  existing 48 MiB job limit. Measured text/positions remain text. Existing SVG
  stroke limitation and `SVG_TEXT_NOT_OUTLINED` diagnostic remain.
- The old bundled-font HTTP URL also serves the compiled pinned bytes. Worker
  job input now uses project-scoped font downloads for both project and bundle.

User/API instructions and explicit limits are in `docs/FONTS.md`.

## Verification actually run

All data roots were temporary, HTTP servers used random loopback ports; no
`~/.opencanvas`, real design, Docker or port 3847 was modified.

| Command | Result |
| --- | --- |
| `cargo test --offline -p opencanvas-storage --test font_portability -j 2` | 2 passed: exact original Noto/Lato reads after store reopen; distinct hashes with same metadata; project bundle restore into independent root; exact-byte second-project import; invalid/stale atomicity; corrupt/unknown read errors |
| `cargo test --offline -p opencanvas-server --test font_portability -- --test-threads=1` | 2 passed: real HTTP import/list/download, bundled/project bytes, MIME/attachment, idempotent replay/events, stale/invalid atomicity, MCP metadata/page, Host/Origin guard, independent project; real production SVG with exact embedded Noto/Lato bytes |
| `cargo test --offline -p opencanvas-server --lib export_fonts -- --test-threads=1` | 1 passed: old computed scene cannot export replaced bytes or changed weight/style |
| `cargo test --offline -p opencanvas-server --lib svg:: -- --test-threads=1` | 11 passed: existing SVG scene/markup tests after removing the generic family fallback |
| `cargo test --offline -p opencanvas-server --test font_portability --test export_svg -- --test-threads=1` | final combined run: 4 passed across both suites |
| `node --test --test-concurrency=1 spikes/pipeline/font-portability.test.mjs` | 1 passed: actual Noto and Lato share declared family/weight/style, retain different measurements, later loads do not alter earlier resources, canvas matches measured widths, strict hash/decode failure |
| `cargo clippy --offline -p opencanvas-storage -p opencanvas-server -p opencanvas-layout --lib -j 2 -- -D warnings` | passed |
| `npm run check:renderer`, `cargo fmt --all --check`, `git diff --check` | passed |
| `cargo build --offline -p opencanvas-layout -p opencanvas-server -j 2`, `npm run build:renderer` | passed |

The SVG server regression invokes `spikes/pipeline/font-svg-offline.mjs` against
its actual produced SVG, in a new offline Chromium context. Both private font
faces report loaded, the two identical Vietnamese strings have different real
font widths, and no HTTP request is made. It does not substitute a hand-written
SVG fixture for production export evidence.

Sandbox loopback binding/browser access required the standard test escalation.
The first browser collision assertion used Canvas's default kerning; matching
production `fontKerning="normal"` made it compare the same shaping settings.
An early-origin rejection test was made small to avoid the HTTP client still
streaming an 875 KiB forbidden request when the guard closes the response.

## Limits and file boundaries

FontFace loading does **not** prove glyph coverage. For a character absent from
the selected file, Chromium can still use platform glyph fallback. A reliable
cmap/shape coverage validator and pinned multi-font fallback chain (P11a) are
not implemented here. This is exact-byte font portability for covered text,
not universal cross-OS pixel equality. Noto/Lato Latin/Vietnamese is exercised;
new real OTF/WOFF2 rendering was not tested, though original format retention
and existing signature validation paths are preserved. Import validates file
signature, and strict rendering validates actual decode/readiness.

No P10c family selector, variable axes, rich text or font deletion was added.
Historical `spikes/rust-export` inputs still contain the older renderer version;
regenerate/update that deferred standalone spike before running it against the
current renderer. Active viewer fixtures are updated by the UI task.

Backend responsibilities are separated into font HTTP metadata/handlers,
storage binary reads, SVG font embedding and shared renderer CSS identity.
Every handwritten source/test file changed by this backend task is under 250
lines (largest new test currently 201 lines). No size exception is needed.
