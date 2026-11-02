//! Fixed, process-local counters: no project IDs, keys, URLs, or credentials.
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

pub static PROBES: AtomicU64 = AtomicU64::new(0);
pub static NODE_QUERIES: AtomicU64 = AtomicU64::new(0);
pub static HITS: AtomicU64 = AtomicU64::new(0);
pub static MISSES: AtomicU64 = AtomicU64::new(0);
pub static FALLBACKS: AtomicU64 = AtomicU64::new(0);
pub static MALFORMED: AtomicU64 = AtomicU64::new(0);
pub static STORES: AtomicU64 = AtomicU64::new(0);
pub static OVERSIZE: AtomicU64 = AtomicU64::new(0);
pub static DISABLED: AtomicU64 = AtomicU64::new(0);
pub static DB_ERRORS: AtomicU64 = AtomicU64::new(0);

pub fn increment(counter: &AtomicU64) {
    counter.fetch_add(1, Ordering::Relaxed);
}

pub fn snapshot() -> Value {
    json!({
        "probes": PROBES.load(Ordering::Relaxed),
        "nodeQueries": NODE_QUERIES.load(Ordering::Relaxed),
        "hits": HITS.load(Ordering::Relaxed),
        "misses": MISSES.load(Ordering::Relaxed),
        "fallbacks": FALLBACKS.load(Ordering::Relaxed),
        "malformed": MALFORMED.load(Ordering::Relaxed),
        "stores": STORES.load(Ordering::Relaxed),
        "oversize": OVERSIZE.load(Ordering::Relaxed),
        "disabled": DISABLED.load(Ordering::Relaxed),
        "dbErrors": DB_ERRORS.load(Ordering::Relaxed),
    })
}
