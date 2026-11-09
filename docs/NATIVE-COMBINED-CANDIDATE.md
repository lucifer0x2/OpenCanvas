# Native combined candidate — 2026-11-09

One candidate carrying both checked deltas, built in
`.runtime/native/combined-20261109/release/`. **Activated on 2026-11-09 at the
user's request** — see [Activation](#activation--2026-11-09). It supersedes installing
`page-concurrency-20261109` and `viewer-cache-20261109` separately; each of those
alone would drop the other feature. No commit or push.

## Baseline and delta

Both earlier candidates reconstructed the same live baseline: Git
`4905dd57050b838ba5aedbfb46b1e5a8179a2f97` + installed `SOURCE_PATCH`
(`33bbe9f7…`) + `FRONTEND_PATCH` (`d31abe4d…`); their `baseline-source/` trees
are recursively identical. The deltas touch disjoint files:

- Page-scoped concurrency ([PAGE-CONCURRENCY.md](PAGE-CONCURRENCY.md)): storage
  `scoped*`, batch/history/backends, MCP schemas/instructions/skill, tests.
- Viewer cache ([VIEWER-CACHE-POLICY.md](VIEWER-CACHE-POLICY.md)): server
  `viewer_static.rs`, `http.rs`/`lib.rs` wiring, cache tests.

`source/` is the page-concurrency source with the four cache files and the cache
fixture copied in. `release/COMBINED_PATCH` (35 files) reapplies with
`patch -p2` to `baseline-source/` and matches `source/` exactly.

The release is an APFS clone of the live-based release with only the native
server binary replaced. All **566 other payload files** match the live manifest
(same fingerprint as the cache candidate). Viewer stays on `index-CZExainG.js`.

| Artifact | SHA256 |
| --- | --- |
| Candidate server | `0434a1203d9169a3d5ac02f2b809c2715499ce73619daeff7835c6081aea2517` |
| Combined source/test patch | `f271654aad80364c8167f663b825fcf87426326eeb45615834fc2b50a43a731b` |
| Preserved-file manifest (live = candidate) | `c24342ab2daf91626477e37a65a128dcca30ad7ee4c984e73b039b85c5ba1dfa` |

`release/PROVENANCE` keeps the original lines and appends feature, cache and
combined patch hashes; `release/SHA256SUMS` was regenerated.

## Verification

`bash .runtime/native/combined-20261109/validate.sh` (fresh PostgreSQL 16 cluster
in a temporary directory on a random loopback port, removed afterwards):

- Storage `scoped_concurrency/history/navigation/bundle --include-ignored`:
  9 passed, PostgreSQL included.
- Server `scoped_concurrency`, `structured_batch`, `tool_schemas`, `tools_budget`,
  `skill_layer`, `viewer_cache_policy`, `public_cli`, `public_viewer`,
  `public_viewer_auth`, `project_privacy`: 26 passed.
- Release build, then `smoke.mjs` on PostgreSQL for both render backends (`rust`,
  `chromium`): independent page commits, stale conflicts, strict legacy mode,
  replay, ETag, hidden history envelope, PNG, fonts, WASM, public isolation — ok.

`node .runtime/native/combined-20261109/canary.mjs` (temporary SQLite workspace):
both listeners, HTML entry/fallback `no-store`, immutable hashed assets, stable
WASM, compression + `Vary`, fresh body after same-mtime replacement, MCP,
read-only public, password cookie and privacy after authentication — ok.

Main checkout (`feat/task09-wasm-spike`) was rechecked separately: fmt clean,
storage scoped/bundle tests 11 passed, server targeted tests 19 passed.
Pre-existing failures unrelated to these deltas: storage
`stroke_props::catalog_lists_stroke_for_box_kinds_only` (Instance now lists
`stroke`) and `clippy --all-targets` lints in unmodified tests
(`states_render.rs`, `instances_mcp.rs`, dead test helpers).

## Activation procedure

Follow the same procedure as the cache candidate: copy the release to a
new installed sibling, e.g.
`4905dd57050b838ba5aedbfb46b1e5a8179a2f97+page-scope.9deddf640f3c+viewer-cache.f8dcc444b1ca`,
validate `SHA256SUMS`, change only `OPENCANVAS_RELEASE` in private
`config/server.env`, gracefully stop the server LaunchAgent, wait for both
listeners to disappear, bootstrap it and check both listeners. PostgreSQL,
backup and proxy keep running.

No schema migration is needed. Commits written by the new binary carry a
versioned private history envelope that older binaries cannot read; rolling back
only `OPENCANVAS_RELEASE` after scoped/new commits exist is therefore not
supported without restoring the pre-activation backup. Take a fresh backup
(`docs/NATIVE-BACKUP.md`) immediately before activation.

After activation, MCP clients must reconnect to pick up the `pageIds` schema and
instructions; browsers holding an old cached HTML may need one hard reload.

## Activation — 2026-11-09

Executed after the user's explicit request (“deploy bản native đi, backup database”).

1. Live release binary and 566-file payload manifest were rechecked against the
   candidate baseline: identical.
2. Manual backup `backups/backup-20261109T052151Z-thM5cl2T/`: fresh scratch
   restore and content verification passed, `SHA256SUMS` OK, 14 sets retained.
3. Installed `runtime/releases/4905dd57050b838ba5aedbfb46b1e5a8179a2f97+native-overlay.33bbe9f75b87+react-singleton.d31abe4d832c+scope-cache.f271654aad80/`
   (APFS clone, mode 0700, `SHA256SUMS` OK, server `0434a120…`).
4. Saved `config/server.env.pre-scope-cache-20261109` (0600); changed only
   `OPENCANVAS_RELEASE`. Server LaunchAgent booted out gracefully (`Stopping…`,
   no old process left) and bootstrapped. PostgreSQL and backup agents untouched.

Post-activation checks:

- All 5 snapshots byte-identical before/after (revisions `8,10,18,60,211`).
- `verify-runtime.mjs`: MCP `2025-11-25`, PNG render, WASM bytes — ok.
- Live MCP lists `apply_batch.pageIds` and `get_edit_context`; a read-only
  `get_edit_context` on PostgreSQL returned a guard and 12 dependencies.
- Local and public HTML/SPA fallback `Cache-Control: no-store`; hashed bundle
  `public, max-age=31536000, immutable`.
- Public 3858 on both approved hosts: HTML 200, projects 401, MCP 403; unknown
  host 403. HTTPS alternate via Cloudflare: HTML `no-store`, `DYNAMIC`. Cloudflare
  rewrites the hashed bundle to `max-age=14400` (its Browser Cache TTL); harmless
  because the file name changes with content.

Rollback: restore `config/server.env.pre-scope-cache-20261109`, then boot out and
bootstrap `org.opencanvas.server`. If scoped/new commits were written after
activation, the old binary cannot read their history envelope: restore the
backup above instead (`docs/NATIVE-BACKUP.md`), accepting loss of later edits.
