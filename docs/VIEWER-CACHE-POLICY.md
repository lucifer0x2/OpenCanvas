# Viewer static cache policy

The local and public viewer share `crates/server/src/viewer_static.rs` through
`http::viewer_routes`. The policy wraps only static serving; API responses,
snapshot ETags, public password/session checks and project privacy keep their
existing behavior. No service worker or browser cache/session deletion is added.

| Static response | Cache-Control | Conditional requests |
| --- | --- | --- |
| `/`, `/index.html`, HTML and SPA fallback | `no-store` | Always fetch the current shell; no mtime/entity validation |
| Stable filenames, including `/scene-wasm/*.wasm` and stable JS/CSS | `no-store` | Always fetch current bytes |
| Existing Vite `/assets/<name>-<8-character-hash>.js` or `.css` | `public, max-age=31536000, immutable` | Existing file conditional GET/HEAD and 304 remain supported |
| Missing files, redirects, errors and other extensions | `no-store` | Never treated as immutable bundles |

The immutable matcher accepts only direct children of `/assets/`, an ASCII
alphanumeric/underscore/hyphen name, and Vite's default eight base64url hash
characters before `.js`/`.css`. It assumes these names come from Vite's
content-hashed build output; deployments must never overwrite a hash URL with
different content. Custom naming formats safely receive `no-store`. Query strings
do not change classification. Successful responses must also have a JS/CSS MIME
type; missing hash-looking paths that return the HTML fallback remain `no-store`.

Stable responses remove `Last-Modified` and `ETag`. Their incoming conditional
headers and ranges are discarded, including inside the SPA fallback, because a
release archive can preserve a file's mtime while replacing its content. Thus an
old `If-Modified-Since` cannot produce a stale-shell 304 after an update. GET
returns the complete current bytes; HEAD uses the same policy and has no body.
The existing SPA fallback status is preserved (currently 404).

Brotli/gzip file negotiation remains enabled, with `Vary: Accept-Encoding` on
static responses including HEAD and 304. Content-hashed assets can be retained
for a year while the entry shell and stable WASM always travel to the server.

Previously cached entries do not acquire these headers retroactively. A browser
must make its next network request (or perform a one-time reload bypassing its
old cache) before the new policy takes effect. Any explicit upstream/CDN cache
override also needs to respect the origin policy. An already open viewer loads
new frontend code when the page is reloaded; this change adds no forced reload.

## Verification

```sh
cargo test --offline -p opencanvas-server --test viewer_cache_policy -j 2 -- --test-threads=1
```

Three tests use temporary workspace/data roots and both listeners. They replace
old HTML/stable assets while preserving mtimes; cover GET/HEAD, conditional
headers, ranges, deep links, query strings, missing hashed assets, MIME/cache
headers, Brotli/gzip negotiation and hashed-bundle 304; and confirm the existing
snapshot ETag response remains local `private, no-cache` / public `no-store`.

## Applying to a newer runtime source tree

Do not replace `http.rs` or a live server with this checkout's older source.
The integration consists of these changes only:

1. Copy `crates/server/src/viewer_static.rs` and register `mod viewer_static;` in
   `crates/server/src/lib.rs`.
2. In `http::viewer_routes`, replace just the static `.fallback_service(...)`
   expression with `.fallback_service(crate::viewer_static::service(&dist))`.
3. Remove the now-unused `ServeDir`/`ServeFile` imports and, where applicable,
   `SetResponseHeader`. The new module includes precompression and Vary handling;
   retain all other newer routes, guards and middleware.
4. Carry over the focused test and fixture module, then run it against that
   source tree before preparing any runtime artifact.

No live runtime, configuration, listener or database is changed by this patch.
