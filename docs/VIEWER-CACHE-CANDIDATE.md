# Native viewer cache candidate — 2026-11-09

Candidate is prepared and independently checked, **not activated**:
`.runtime/native/viewer-cache-20261109/release/`.
No installed release, private configuration, live database, service or public
proxy was modified. Canary listeners stopped; temporary SQLite/workspaces were
removed. No commit or push.

## Exact live baseline and delta

Reconstructed source from Git `4905dd57050b838ba5aedbfb46b1e5a8179a2f97`,
then applied the installed release's `SOURCE_PATCH` (SHA256
`33bbe9f75b877a9cfd80231cd85e135d2fb861bb199542f7250d3f7b9b884794`)
and `FRONTEND_PATCH` (SHA256
`d31abe4d832ce3bd1249070912faaad228d0d10990b5ff01eac608755eec74f8`).
The reconstructed baseline also matches the earlier independently reconstructed
page-concurrency baseline; no page-concurrency changes entered this candidate.

Added only the shared `viewer_static` policy module and surgical module/fallback
wiring to the live `lib.rs`/`http.rs`. Retained the newer `/scenes` route, scene
and render implementation, MCP, auth/privacy/domain overlay and compressed
static serving. Copied the regression tests and their focused fixture helper.
The complete delta is `release/CACHE_PATCH`; `patch -p2` reapplies it exactly to
`baseline-source/`, verified by a clean recursive diff.

The release is an APFS clone of the installed release with only its native
server binary replaced. No shared hardlink was modified. All **566 other
payload files**, including web/WASM/worker/renderer/layout/Node/Playwright/browser
bytes, match live. The viewer stays on `index-CZExainG.js` with the React singleton
fix. `release/SHA256SUMS` was regenerated, original provenance retained and cache
delta provenance appended.

| Artifact | SHA256 |
| --- | --- |
| Candidate server | `02879a16531989abe535165e9fe1244032adfac18b905eecc4f08f250aa5d032` |
| Cache source/test patch | `f8dcc444b1ca7e502eb62dcbe084408bd702be12f31fb2093bee5e6add2d3abd` |
| Sorted preserved-file manifest, both live and candidate | `c24342ab2daf91626477e37a65a128dcca30ad7ee4c984e73b039b85c5ba1dfa` |

## Actual verification

Commands ran from the repository, with `CARGO_TARGET_DIR` set to
`$PWD/.runtime/native/viewer-cache-20261109/target`. The target is a separate APFS
clone of the prior live build cache; the original cache was not written.

```sh
cargo build --manifest-path .runtime/native/viewer-cache-20261109/source/Cargo.toml \
  --offline --locked --release -p opencanvas-server --bin opencanvas-server -j 2
cargo test --manifest-path .runtime/native/viewer-cache-20261109/source/Cargo.toml \
  --offline --locked -p opencanvas-server --test viewer_cache_policy \
  --test public_viewer_auth --test project_privacy --test public_cli \
  -j 2 -- --test-threads=1
cargo fmt --manifest-path .runtime/native/viewer-cache-20261109/source/Cargo.toml \
  -p opencanvas-server -- --check
node .runtime/native/viewer-cache-20261109/canary.mjs --baseline
node .runtime/native/viewer-cache-20261109/canary.mjs
```

Release build passed in 32.21 seconds; targeted tests **12/12 passed**: cache 3,
auth 3, privacy 4, CLI/lifecycle 2. Formatting, shell syntax and Node syntax checks
passed. Initial sandbox loopback binding failed with EPERM; canary/tests then
passed with approved loopback escalation. No live configuration was sourced.

Baseline canary reproduced stale HTML after replacing `index.html` while
preserving mtime: `/` and `/index.html` returned stale-validator 304; SPA fallback
retained its existing 404 with an empty conditional body. Patched canary returned
fresh bodies with `no-store` on both listeners, including the existing SPA 404.
It sent the actual old second-resolution mtime as `If-Modified-Since`.

Checks also passed for hashed JS/CSS immutable caching, real stable WASM bytes,
exact precompressed Brotli/gzip bytes and `Vary: Accept-Encoding`, local/public
`readOnly`, local snapshot ETag/304, MCP protocol negotiation, host/origin guards,
public mutation/MCP denial, optional password cookie, host-bound sessions and
private-project exclusion after authentication. All project/auth mutations were
against temporary SQLite only, with `example.test` origins. Parent reran the
packaged release canary independently and confirmed success and cleanup.

Production files are 110 lines (`viewer_static`), 195 (`http`) and 48 (`lib`);
test/fixture files are 196/75 lines. Each has one responsibility; none exceeds
250 lines. Logs/manifests/patches and tooling remain under the ignored candidate
directory; this report is the only tracked output from candidate preparation.

## Activation and rollback proposal — not executed

After explicit approval, copy the completed release to a new installed sibling,
for example `4905dd57050b838ba5aedbfb46b1e5a8179a2f97+viewer-cache.f8dcc444b1ca`.
Validate its manifest and privately preserve the existing `OPENCANVAS_RELEASE`
value. Change only that assignment in private `config/server.env`; retain all
database/auth/origin/proxy settings. Gracefully stop the server LaunchAgent,
wait until its process and both listeners disappear, then bootstrap it and check
both listeners and static policies. PostgreSQL and proxy remain running.

For rollback, gracefully stop the server, restore only `OPENCANVAS_RELEASE` to
the retained `4905dd57050b838ba5aedbfb46b1e5a8179a2f97+native-overlay.33bbe9f75b87+react-singleton.d31abe4d832c`
release and bootstrap. Keep the current native PostgreSQL database; no migration
or database rollback is needed. Retain both releases until acceptance.

Limits: no production activation or HTTPS edge/browser acceptance was performed;
no PostgreSQL test was needed for this static-only delta. A browser that already
holds an unexpired old HTML response may need one reload that bypasses cache to
receive the new policy; server headers cannot revoke an already cached response.
