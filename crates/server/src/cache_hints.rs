//! SEP-2549 cache hints on the MCP catalog results this server publishes.
//!
//! Protocol 2026-07-28 requires `ttlMs` and `cacheScope` on list and read results, and a client
//! rejects a result without them. Older revisions accept them as extra result properties and
//! rmcp does not strip them, so every negotiated version receives the same values.
use rmcp::model::{
    CacheScope, ListResourceTemplatesResult, ListResourcesResult, ListToolsResult,
    ReadResourceResult,
};

/// The tool catalog and the embedded skill resources are fixed for the process lifetime.
const TTL_MS: u64 = 300_000;
/// A local server answering for one user: no intermediary may share its results.
const SCOPE: CacheScope = CacheScope::Private;

/// Applies the catalog cache policy to one result.
pub(crate) trait Cached {
    fn cached(self) -> Self;
}

// rmcp gives each result type the same two builder methods but no shared trait.
macro_rules! cached {
    ($($result:ty),+) => {$(
        impl Cached for $result {
            fn cached(self) -> Self {
                self.with_ttl_ms(TTL_MS).with_cache_scope(SCOPE)
            }
        }
    )+};
}
cached!(
    ListToolsResult,
    ListResourcesResult,
    ListResourceTemplatesResult,
    ReadResourceResult
);
