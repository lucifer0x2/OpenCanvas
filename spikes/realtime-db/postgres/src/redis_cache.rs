//! Bounded Redis transport and integrity-checked encoded response entries.
//! SHA-256 detects corruption/binding errors; it is not an authorization mechanism.
use crate::{cache_metrics as metrics, hot_snapshot};
use redis::{aio::MultiplexedConnection, AsyncConnectionConfig};
use sha2::{Digest, Sha256};
use std::time::Duration;
use tokio::sync::{Mutex, Semaphore};

const MAGIC: &[u8] = b"OCS1";
const HEADER: usize = 36;
const BOUNDED_GET: &str = "if redis.call('STRLEN',KEYS[1])>tonumber(ARGV[1]) then return false end return redis.call('GET',KEYS[1])";

pub struct RedisCache {
    client: redis::Client,
    connection: Mutex<Option<MultiplexedConnection>>,
    slots: Semaphore,
    namespace: String,
    ttl_seconds: u64,
    max_bytes: usize,
    timeout: Duration,
}

impl RedisCache {
    pub fn from_env(pg_url: &str) -> Option<Self> {
        let redis_url = std::env::var("REDIS_URL").ok()?;
        let result = Self::new(
            pg_url,
            &redis_url,
            setting("REDIS_CACHE_TTL_SECONDS", 3600, 3600),
            setting("REDIS_CACHE_MAX_BYTES", 8 * 1024 * 1024, 16 * 1024 * 1024) as usize,
            Duration::from_millis(setting("REDIS_CACHE_TIMEOUT_MS", 100, 2000)),
        );
        if result.is_none() {
            eprintln!("redis_cache_config_invalid");
        }
        result
    }

    pub fn new(
        pg_url: &str,
        redis_url: &str,
        ttl_seconds: u64,
        max_bytes: usize,
        timeout: Duration,
    ) -> Option<Self> {
        Some(Self {
            client: redis::Client::open(redis_url).ok()?,
            connection: Mutex::new(None),
            slots: Semaphore::new(16),
            namespace: hot_snapshot::namespace(pg_url),
            ttl_seconds: ttl_seconds.clamp(1, 3600),
            max_bytes: max_bytes.clamp(1, 16 * 1024 * 1024),
            timeout: timeout.clamp(Duration::from_millis(1), Duration::from_secs(2)),
        })
    }

    pub fn namespace(&self) -> &str {
        &self.namespace
    }

    pub async fn get(&self, key: &str) -> Option<Vec<u8>> {
        let mut command = redis::cmd("EVAL");
        command
            .arg(BOUNDED_GET)
            .arg(1)
            .arg(key)
            .arg(self.max_bytes + HEADER);
        match self.operation::<Option<Vec<u8>>>(command).await {
            Some(Some(mut entry)) => {
                if entry.len() < HEADER
                    || !entry.starts_with(MAGIC)
                    || entry[4..HEADER] != digest(key, &entry[HEADER..])
                {
                    metrics::increment(&metrics::MALFORMED);
                    metrics::increment(&metrics::MISSES);
                    return None;
                }
                metrics::increment(&metrics::HITS);
                entry.drain(..HEADER);
                Some(entry)
            }
            Some(None) => {
                metrics::increment(&metrics::MISSES);
                None
            }
            None => None,
        }
    }

    pub async fn put(&self, key: &str, bytes: &[u8]) {
        if bytes.len() > self.max_bytes {
            metrics::increment(&metrics::OVERSIZE);
            return;
        }
        let mut entry = Vec::with_capacity(HEADER + bytes.len());
        entry.extend_from_slice(MAGIC);
        entry.extend_from_slice(&digest(key, bytes));
        entry.extend_from_slice(bytes);
        let mut command = redis::cmd("SET");
        command.arg(key).arg(entry).arg("EX").arg(self.ttl_seconds);
        if self.operation::<()>(command).await.is_some() {
            metrics::increment(&metrics::STORES);
        }
    }

    async fn operation<T: redis::FromRedisValue>(&self, command: redis::Cmd) -> Option<T> {
        let Ok(_permit) = self.slots.try_acquire() else {
            metrics::increment(&metrics::FALLBACKS);
            return None;
        };
        let result = tokio::time::timeout(self.timeout, async {
            let mut connection = {
                let mut cached = self.connection.lock().await;
                if cached.is_none() {
                    let config = AsyncConnectionConfig::new()
                        .set_connection_timeout(Some(self.timeout))
                        .set_response_timeout(Some(self.timeout));
                    *cached = Some(
                        self.client
                            .get_multiplexed_async_connection_with_config(&config)
                            .await?,
                    );
                }
                cached.as_ref().unwrap().clone()
            };
            command.query_async::<T>(&mut connection).await
        })
        .await;
        match result {
            Ok(Ok(value)) => Some(value),
            _ => {
                // The lock is never held over SQL. Drop broken/timed-out transports
                // so a restart can recover without restarting the gateway.
                if let Ok(mut cached) = self.connection.try_lock() {
                    *cached = None;
                }
                metrics::increment(&metrics::FALLBACKS);
                None
            }
        }
    }
}

fn digest(key: &str, bytes: &[u8]) -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(key.as_bytes());
    hash.update(bytes);
    hash.finalize().into()
}

fn setting(name: &str, default: u64, maximum: u64) -> u64 {
    std::env::var(name)
        .ok()
        .and_then(|v| v.parse().ok())
        .unwrap_or(default)
        .clamp(1, maximum)
}
