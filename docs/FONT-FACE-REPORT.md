# P10c static font faces — 2026-10-30

Implemented exact imported-face selection in storage after text-style merge and
theme token resolution. Family/style are literal selectors; weight also accepts
number tokens and must resolve to an integer 100–900 in every theme. Direct node
fontId/family selects the base before inherited weight/style overrides. Missing
faces fail with imported alternatives, unknown family includes a case-insensitive
suggestion, and duplicate faces retain an explicit matching base or reject. No
nearest-weight lookup or synthesized bold/italic is introduced. Legacy fontId-only
resolved props retain their original shape.

Catalog, value validation and required-font checks are integrated by the text
foundation worker. The resolved selected fontId is consumed by the existing
portable-byte rendering pipeline. Compact context retains authored references and
effective faces across all token themes; reference mode also includes faces needed
by master definitions and instance style overrides. Expanded-only context carries
the selected instance face without unrelated definitions. Original content hashes
remain in resource records and scene manifests.

## Checks actually run

- `cargo test --offline -p opencanvas-storage --test font_face --test resolution --test style_aliases --test prop_catalog -- --test-threads=2`:
  17 passed, including exact selection, direct/style precedence, required selectors,
  duplicate/missing faces, invalid scalars, case suggestion, old-props equality,
  themed weight tokens and instance style overrides. Catalog B02 passes following
  the text worker's authored-key precheck.
- `cargo test --offline -p opencanvas-storage --test font_face --test resolution --test style_aliases --test instances --test prop_catalog -- --test-threads=2`:
  face3 and existing instances4 passed before the catalog error stopped that earlier
  combined run. The later command above verifies the corrected catalog.
- `cargo test --offline -p opencanvas-server --test font_face_context --test compact_context -- --test-threads=1`:
  7 passed. This was before the additional offline SVG-open assertions.
- `cargo test --offline -p opencanvas-server --test font_face_context -- --test-threads=1`:
  2 passed after adding offline SVG-open assertions. Temporary port0 servers and
  temporary data roots only; server tasks and roots clean up through the harness.
  Regular/Bold/Italic changes produce three distinct PNGs and scene cache keys;
  font IDs, weight/style and exact hashes reach scene manifests. Style-only changes
  select a new face at the new revision. Missing italic700 rejects atomically.
- The same integration test opens three real SVG exports in a fresh offline
  Chromium context via `spikes/pipeline/font-face-svg-offline.mjs`: all embedded
  faces load under hash-derived families, zero HTTP requests, three distinct pixel
  hashes and differing regular/bold text widths. SVG embeds the original static
  file bytes, and downloaded face bytes equal their fixtures byte-for-byte.
- `cargo clippy --offline -p opencanvas-storage -p opencanvas-server --lib --bin opencanvas-server -- -D warnings`: passed.
- `rustfmt --edition 2021 --check` on the six owned Rust implementation/test files
  and `node --check spikes/pipeline/font-face-svg-offline.mjs`: passed.

Static fixtures are unmodified Google Fonts Lato Regular400, Bold700 and
Italic400, pinned to commit `f3b885d5590e307e02542f1a724cec55d567fdaa`.
Source/license/hash details: [fixtures/fonts/README.md](../fixtures/fonts/README.md).
The storage metadata fixtures also use the pinned bundled Noto bytes; real render
evidence uses distinct correctly labeled static Lato files.

## Scope and sizes

Owned production modules: face resolver143 lines, refs198, compact140, tree214.
Tree retains the existing read orchestration with one fallible compact-collection
call. Tests203/206 lines retain coherent table-driven acceptance scenarios; no
owned file exceeds250 lines. Shared catalog/validation integration is documented
in the text foundation report. AGENTS.md remains64 lines/7991 bytes.

This lane did not run the full workspace suite or restart3847. Global formatting,
UI/preview typography consumers and consumer-version changes are coordinated by
the root and other lanes. Variable axes and rich text remain outside P10c.
