# Project fonts

OpenCanvas uses uploaded project bytes or its pinned bundled Noto Sans TTF. Design
text does not search installed OS fonts by family or load remote Google Fonts at
runtime. `renderer/fonts/manifest.json` records the existing Google Fonts source,
SHA-256 and OFL license. The bundled font was already present; this change does
not replace project designs or add a runtime font catalogue.

Each font record selects one face by `fontId`, with readable `family`, numeric
`weight` (100–900) and `style` (`normal` or `italic`). Family metadata does not
select an installed font. Import each actual required face; changing metadata
does not manufacture a bold/italic face or verify its internal font names.

Text and text styles can select `fontFamily` with `fontWeight` and `fontStyle`
instead of remembering each face ID. Selection requires an exact imported face;
missing and ambiguous matches return errors. An explicit matching `fontId` can
disambiguate duplicates. Weight accepts a number token and resolves per theme.
See [face resolution](FONT-FACE-REPORT.md) for precedence and compatibility.

## Import and download

In the browser, open **Fonts** in the selected project's header to add a font file,
enter its family/weight/style and optional license, or download an original file.
No operating-system installation is needed. This adds a project resource; existing
text keeps its assigned font. See [the viewer guide](VIEWER.md#project-fonts) for
keyboard controls, validation and safe retry behavior.

`GET /api/v1/projects/{project}/fonts` returns:

```json
{"projectId":"p_…","revision":1,"fonts":[{"id":"body","family":"My Sans","weight":400,"style":"normal","assetId":"asset_…","contentHash":"…","status":"ready","mediaType":"font/ttf","byteSize":12345,"source":"project","downloadUrl":"http://127.0.0.1:…/api/v1/projects/p_…/fonts/body/download"}]}
```

Existing font fields remain present; `source` is `project` or `bundled`.
The inventory is limited to 8 MiB; MCP `list_fonts` provides revision-bound
pagination and the same additional metadata when a smaller page is needed.

`POST /api/v1/projects/{project}/fonts` accepts:

```json
{"expectedRevision":0,"idempotencyKey":"stable-import-key","mediaType":"font/ttf","base64":"<original file bytes encoded as base64>","font":{"id":"body","family":"My Sans","weight":400,"style":"normal","license":"OFL-1.1"}}
```

Supported original formats are TTF (`font/ttf`), OTF (`font/otf`) and WOFF2
(`font/woff2`), bounded to 10 MiB decoded. The server checks the matching binary
signature; actual font decoding/readiness occurs during rendering. A signed but
malformed font cannot produce a successful strict render. The HTTP route uses
the existing MCP `import_asset` transaction, revision conflict handling,
idempotency replay and commit events. Retry the same body/key after a lost
response; use a new key after changing the request. Invalid requests and stale
revisions cannot partially register a font or advance the revision.

`GET /api/v1/projects/{project}/fonts/{fontId}/download` returns exact original
bytes with the original MIME type and a safe attachment filename ending in
`.ttf`, `.otf` or `.woff2`. It also exports `font_bundled` as its exact pinned
TTF. There is no format conversion. Missing files, unknown IDs and corrupted
bytes return errors, rather than loading another font. Downloads use checked
project assets, never client filesystem paths. Loopback Host/Origin checks apply;
only resource GET permits the renderer's random loopback origin. Mutations
require the configured origin when an Origin header is supplied.

For MCP, use `import_asset` with `font` registration; `list_fonts` now provides
the download URLs. No additional duplicate import tool is needed. For another
project/machine, download each font and import the same bytes there, or use the
existing project bundle export/import, which includes uploaded and bundled
font bytes and the bundled OFL license. Copying `db.sql` alone is insufficient:
uploaded font bytes live under the project's `assets/`. Keep applicable licenses
with fonts shared outside the bundle.

## Rendering guarantees and limits

Measurement, canvas drawing, preview SVG text and exported SVG use a private CSS
family derived from SHA-256, weight and style. Two different files registered
under the same readable family can coexist; loading one no longer replaces the
other. Authored metadata stays unchanged. SHA verification and awaited FontFace
readiness precede strict rendering. Renderer identity is `web-4`, coordinated
with layout/cache keys; rebuild renderer/layout/server together and restart a
backend to use the new pipeline.

Production SVG export embeds the exact font bytes as data `@font-face` rules.
It checks the stored font against the computed scene's hash, weight and style
and limits embedded font CSS to 40 MiB within the existing 48 MiB export-job
limit. SVG retains measured line text and positions; it does not outline text.
The existing `SVG_TEXT_NOT_OUTLINED` and stroke-export limitation remain.

**Coverage diagnostics:** supported SFNT Unicode cmap tables are checked against
the text. `MISSING_GLYPH` identifies absent scalar mappings; unsupported font
containers/tables report `GLYPH_CHECK_UNAVAILABLE`. Neither FontFace readiness
nor cmap mappings prove complex cluster, variation-selector or emoji shaping.
These are advisories: Chromium can still use platform glyph fallback for missing
characters. There is no pinned multi-font fallback chain. Provide fonts covering
the actual text and validate representative output. Same font bytes do not
guarantee equal pixels across browser versions, shaping engines or operating systems.
The regression suite covers real Latin/Vietnamese text in Noto Sans and Lato;
real OTF/WOFF2 rendering has not been newly exercised by this task.
