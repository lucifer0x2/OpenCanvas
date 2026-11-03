//! Cache is read-only derivation: writers never invoke this module.
use crate::{cache_metrics, hot_snapshot, redis_cache::RedisCache};
use oc_spike_core::DomainError;

pub async fn snapshot(
    url: &str,
    id: &str,
    cache: Option<&RedisCache>,
) -> Result<Vec<u8>, DomainError> {
    snapshot_interleaved(url, id, cache, || async {}).await
}

pub(crate) async fn snapshot_interleaved<F, Fut>(
    url: &str,
    id: &str,
    cache: Option<&RedisCache>,
    after_probe: F,
) -> Result<Vec<u8>, DomainError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    let result = read(url, id, cache, after_probe).await;
    if result.is_err() {
        cache_metrics::increment(&cache_metrics::DB_ERRORS);
    }
    result
}

async fn read<F, Fut>(
    url: &str,
    id: &str,
    cache: Option<&RedisCache>,
    after_probe: F,
) -> Result<Vec<u8>, DomainError>
where
    F: FnOnce() -> Fut,
    Fut: std::future::Future<Output = ()>,
{
    if let Some(cache) = cache {
        // Never trust a cached latest pointer: this indexed DB read is mandatory.
        // Release its DB lease before waiting on Redis or beginning a cold read.
        let identity = hot_snapshot::probe(url, id).await?;
        let key = identity.key(cache.namespace(), id);
        after_probe().await;
        if let Some(bytes) = cache.get(&key).await {
            return Ok(bytes);
        }
    } else {
        cache_metrics::increment(&cache_metrics::DISABLED);
        after_probe().await;
    }
    let (identity, value) = hot_snapshot::load(url, id).await?;
    let bytes = serde_json::to_vec(&value).map_err(crate::db::internal)?;
    if let Some(cache) = cache {
        // A concurrent commit after the first probe may change the cold snapshot.
        // Always populate the actual revision/generation read by the transaction.
        let key = identity.key(cache.namespace(), id);
        cache.put(&key, &bytes).await;
    }
    Ok(bytes)
}
