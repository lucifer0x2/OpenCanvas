// Explicit real-DB suite: PG_URL is required; do not silently report a skipped pass.
#[path = "contract/atomic.rs"]
mod atomic;
#[path = "../src/batch.rs"]
mod batch;
#[path = "../src/batch_write.rs"]
mod batch_write;
#[path = "../src/blobs.rs"]
mod blobs;
#[path = "contract/bulk_failure.rs"]
mod bulk_failure;
#[path = "contract/bulk_nodes.rs"]
mod bulk_nodes;
#[path = "contract/bulk_replay.rs"]
mod bulk_replay;
#[path = "../src/cache_metrics.rs"]
mod cache_metrics;
#[path = "../src/compatibility.rs"]
mod compatibility;
#[path = "../src/db.rs"]
mod db;
#[path = "../src/delta.rs"]
mod delta;
#[path = "contract/delta.rs"]
mod delta_contract;
#[path = "contract/delta_failures.rs"]
mod delta_failures;
#[path = "../src/durable_http.rs"]
mod durable_http;
#[path = "../src/events.rs"]
mod events;
#[path = "contract/fast_paths.rs"]
mod fast_paths;
#[path = "../src/hot_reads.rs"]
mod hot_reads;
#[path = "contract/hot_reads.rs"]
mod hot_reads_contract;
#[path = "contract/hot_reads_races.rs"]
mod hot_reads_races;
#[path = "../src/hot_snapshot.rs"]
mod hot_snapshot;
#[path = "../src/http.rs"]
mod http;
#[path = "../src/jobs.rs"]
mod jobs;
#[path = "../src/leases.rs"]
mod leases;
#[path = "contract/notifications.rs"]
mod notifications;
#[path = "../src/profile.rs"]
mod profile;
#[path = "contract/reads.rs"]
mod reads;
#[path = "../src/redis_cache.rs"]
mod redis_cache;
#[path = "../src/staging.rs"]
mod staging;
#[path = "contract/support.rs"]
mod support;
#[path = "../src/transport.rs"]
mod transport;

pub struct App {
    url: String,
    cache: Option<redis_cache::RedisCache>,
    wake: tokio::sync::Notify,
}
