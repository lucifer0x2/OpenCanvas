use super::support::*;
use crate::{cache_metrics, db, hot_reads, hot_snapshot, redis_cache::RedisCache};
use serde_json::Value;
use std::{sync::Arc, time::Duration};

pub fn cache(url: &str) -> RedisCache {
    RedisCache::new(
        url,
        &std::env::var("REDIS_URL").expect("REDIS_URL required: isolated Redis"),
        3600,
        8 * 1024 * 1024,
        Duration::from_millis(100),
    )
    .unwrap()
}

pub async fn redis() -> redis::aio::MultiplexedConnection {
    redis::Client::open(std::env::var("REDIS_URL").unwrap())
        .unwrap()
        .get_multiplexed_async_connection()
        .await
        .unwrap()
}

pub async fn read(url: &str, id: &str, cache: &RedisCache) -> Value {
    serde_json::from_slice(&hot_reads::snapshot(url, id, Some(cache)).await.unwrap()).unwrap()
}

#[tokio::test]
async fn cold_then_hot_get_skips_locked_nodes_with_cold_positive_control() {
    let (url, id) = project("cache_hot").await;
    seed(&url, &id).await;
    let cache = cache(&url);
    let cold = read(&url, &id, &cache).await;
    let key = hot_snapshot::probe(&url, &id)
        .await
        .unwrap()
        .key(cache.namespace(), &id);
    let ttl: i64 = redis::cmd("TTL")
        .arg(&key)
        .query_async(&mut redis().await)
        .await
        .unwrap();
    assert!((3590..=3600).contains(&ttl));
    let mut locker = db::connect(&url).await.unwrap();
    let tx = locker.transaction().await.unwrap();
    tx.batch_execute("LOCK TABLE nodes IN ACCESS EXCLUSIVE MODE")
        .await
        .unwrap();
    let hot = tokio::time::timeout(Duration::from_secs(1), read(&url, &id, &cache))
        .await
        .expect("hot GET must not SELECT locked nodes");
    assert_eq!(hot, cold);
    assert!(
        tokio::time::timeout(
            Duration::from_millis(150),
            hot_reads::snapshot(&url, &id, None)
        )
        .await
        .is_err(),
        "positive control: an uncached snapshot must wait for the nodes lock"
    );
    tx.rollback().await.unwrap();
    assert_eq!(read(&url, &id, &cache).await, cold);
    clean(&url, &id).await;
}

#[tokio::test]
async fn malformed_evicted_wrong_binding_and_oversized_entries_fall_back() {
    let (url, id) = project("cache_bad").await;
    assert!(RedisCache::from_env(&url).is_some());
    seed(&url, &id).await;
    let cache = cache(&url);
    let expected = read(&url, &id, &cache).await;
    let key = hot_snapshot::probe(&url, &id)
        .await
        .unwrap()
        .key(cache.namespace(), &id);
    let mut redis = redis().await;
    for corrupt in [b"not json".as_slice(), b"OCS1short".as_slice()] {
        redis::cmd("SET")
            .arg(&key)
            .arg(corrupt)
            .query_async::<()>(&mut redis)
            .await
            .unwrap();
        assert_eq!(read(&url, &id, &cache).await, expected);
    }
    let copied: Vec<u8> = redis::cmd("GET")
        .arg(&key)
        .query_async(&mut redis)
        .await
        .unwrap();
    let other_key = format!("{key}:wrong_binding");
    redis::cmd("SET")
        .arg(&other_key)
        .arg(copied)
        .query_async::<()>(&mut redis)
        .await
        .unwrap();
    assert!(cache.get(&other_key).await.is_none());
    redis::cmd("DEL")
        .arg(&key)
        .arg(&other_key)
        .query_async::<i64>(&mut redis)
        .await
        .unwrap();
    assert_eq!(read(&url, &id, &cache).await, expected);
    let tiny = RedisCache::new(
        &url,
        &std::env::var("REDIS_URL").unwrap(),
        3600,
        8,
        Duration::from_millis(100),
    )
    .unwrap();
    redis::cmd("SET")
        .arg(&key)
        .arg(vec![b'x'; 1024])
        .query_async::<()>(&mut redis)
        .await
        .unwrap();
    assert_eq!(read(&url, &id, &tiny).await, expected);
    assert!(tiny.get(&key).await.is_none());
    clean(&url, &id).await;
}

#[tokio::test]
async fn concurrent_unavailable_redis_is_bounded_and_pg_remains_authoritative() {
    let (url, id) = project("cache_outage").await;
    seed(&url, &id).await;
    // Accept TCP but never answer the Redis handshake: every request must time out.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let redis_url = format!("redis://{}", listener.local_addr().unwrap());
    let sockets = tokio::spawn(async move {
        let mut held = Vec::new();
        loop {
            held.push(listener.accept().await.unwrap().0);
        }
    });
    let cache =
        Arc::new(RedisCache::new(&url, &redis_url, 3600, 4096, Duration::from_millis(50)).unwrap());
    let before = cache_metrics::snapshot();
    let mut tasks = tokio::task::JoinSet::new();
    for _ in 0..40 {
        let cache = cache.clone();
        tasks.spawn(async move {
            assert!(cache.get("unavailable").await.is_none());
        });
    }
    tokio::time::timeout(Duration::from_millis(300), async {
        while let Some(result) = tasks.join_next().await {
            result.unwrap();
        }
    })
    .await
    .expect("fallback must not await the connection mutex after its deadline");
    assert!(
        cache_metrics::snapshot()["fallbacks"].as_u64().unwrap()
            >= before["fallbacks"].as_u64().unwrap() + 40
    );
    let result = tokio::time::timeout(Duration::from_secs(1), read(&url, &id, &cache))
        .await
        .unwrap();
    assert_eq!(result["revision"], 1);
    assert_eq!(result["nodes"].as_array().unwrap().len(), 2);
    sockets.abort();
    let _ = sockets.await;
    clean(&url, &id).await;
}
