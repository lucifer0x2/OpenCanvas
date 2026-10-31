# Project fonts UI — 2026-10-30

The header's **Fonts** action manages resources for the selected project. The canvas
remains read only. The modal lists registered family/weight/style/license and project
or bundled source, imports TTF/OTF/WOFF2 (maximum 10 MiB), and downloads each original
font file through the real attachment endpoint. Bundled OpenCanvas Noto Sans is
downloadable. Font IDs are generated internally; imports never replace existing text
or install anything on the operating system. No system-font enumeration, runtime CDN,
or unverified font specimen is used.

The form checks extension, signature, empty/oversized files, required family and the
backend's 128 UTF-8 byte family limit. Weight/style use the existing custom combobox.
An ambiguous response retains the exact request, ID and idempotency key for an
explicit retry; a revision conflict clears that request, refreshes authoritative data
and requires a fresh user submission. Component lifetimes are keyed by project and
abort previous project requests. Closing/reopening within one project preserves an
unresolved import so its retry remains safe.

Native dialog semantics make the background inert. Explicit first/last Tab wrapping,
Escape and Close restore focus to the trigger. CustomSelect portals inside its
nearest dialog to remain on the modal layer. Controls retain visible focus, labels,
inline errors and loading status. The modal scrolls within small viewports.

## Validation actually run

From `/Users/luis/Workspace/OpenCanvas`:

```sh
npm --prefix web run typecheck
npm --prefix web test
node --test --test-concurrency=1 web/tests/font-files.test.ts
node web/tests/smoke.mjs
OPENCANVAS_FONT_ARTIFACTS=/private/tmp/opencanvas-font-ui-evidence node web/tests/font-ui-smoke.mjs
cd web
node node_modules/vite/bin/vite.js build --outDir /private/tmp/opencanvas-font-ui-dist
```

- TypeScript and production Vite build passed. Vite retains its existing chunk-size
  advisory. Output went to `/private/tmp`; production `web/dist` was not rewritten.
- Existing 23 viewer/model unit tests and the file-bound/signature unit test passed.
- Existing viewer browser smoke passed, including custom combobox click, keyboard,
  typeahead, focus, outside click, Tab, empty state, narrow layout and preview controls.
  Fixture renderer identities were updated to web-3 with the backend worker's approval.
- Actual font browser acceptance used Chromium 156.0.8078.4, Vite's real API proxy and
  production Rust backend on ephemeral ports with a temporary data root. It verified:
  bundled font download and imported font download match the original Noto TTF SHA256;
  import appears in the authoritative API and survives refresh/backend restart;
  retry after an actual committed-but-lost response imports once with identical body;
  actual revision conflict then explicit fresh submit preserves the earlier font;
  invalid extension/signature/oversized file rejection; custom style combobox inside
  modal; forward/backward Tab containment; Escape/focus return; 390px horizontal
  bounds; project switch clears the form/list and leaves the other project unchanged.
- No unhandled browser page errors. The acceptance intentionally causes one interrupted
  request and one HTTP409 to exercise recovery; those expected network errors are not
  application failures. No other console errors are accepted by the smoke.

The sandbox initially blocked loopback listeners (`EPERM`), so browser scripts ran
outside the sandbox. Initial runs identified and corrected Tab escaping to browser
chrome and a test that supplied only a partial node-link URL. The final test uses
the actual project picker. All temporary backend/browser/Vite processes and data roots
are cleaned by `finally`. No real data, live server, OS font install, commit or push.

## Evidence and boundaries

Screenshots (visually inspected):

- `/private/tmp/opencanvas-font-ui-evidence/font-desktop.png`
- `/private/tmp/opencanvas-font-ui-evidence/font-narrow-library.png`
- `/private/tmp/opencanvas-font-ui-evidence/font-narrow.png` (form scrolled to bottom)

The browser smoke imports the pinned real TTF fixture; OTF/WOFF2 extension/signature
validation is covered by unit checks. This UI does not edit node font assignments,
delete fonts or provide a bulk-download button. Backend/export font portability and
byte embedding have their own evidence in `docs/FONTS.md`.

New frontend modules are split by responsibility: types, bounded file handling,
HTTP API, request lifecycle hook, manager, list, form and stylesheet. Topbar was
extracted to keep `App.tsx` at 219 lines. Every new handwritten module and smoke
helper stays below250 lines; `fonts.css` is201 lines, `useFonts.ts`140,
`FontImportForm.tsx`168, `FontManager.tsx`131, and the main smoke remains below250.
`AGENTS.md` remains64 lines/7991 bytes.
