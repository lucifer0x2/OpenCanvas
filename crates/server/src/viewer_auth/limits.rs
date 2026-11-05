use std::{
    collections::HashMap,
    net::IpAddr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::sync::{OwnedSemaphorePermit, Semaphore};

const WINDOW: Duration = Duration::from_secs(60);
const PER_IP: u32 = 5;
const TOTAL: u32 = 60;

pub(crate) struct Limits {
    attempts: Mutex<Attempts>,
    hash: Arc<Semaphore>,
    admin_hash: Arc<Semaphore>,
}

struct Attempts {
    start: Instant,
    total: u32,
    per_ip: HashMap<IpAddr, u32>,
}

impl Limits {
    pub(crate) fn new() -> Self {
        Self {
            attempts: Mutex::new(Attempts::new()),
            hash: Arc::new(Semaphore::new(2)),
            admin_hash: Arc::new(Semaphore::new(1)),
        }
    }

    pub(super) fn admit(&self, ip: IpAddr) -> Option<OwnedSemaphorePermit> {
        let mut attempts = self.attempts.lock().ok()?;
        if attempts.start.elapsed() >= WINDOW {
            *attempts = Attempts::new();
        }
        if attempts.total >= TOTAL || attempts.per_ip.get(&ip).is_some_and(|n| *n >= PER_IP) {
            return None;
        }
        let permit = self.hash.clone().try_acquire_owned().ok()?;
        *attempts.per_ip.entry(ip).or_default() += 1;
        attempts.total += 1;
        Some(permit)
    }

    pub(super) fn try_admin_hash(&self) -> Option<OwnedSemaphorePermit> {
        self.admin_hash.clone().try_acquire_owned().ok()
    }
}

impl Attempts {
    fn new() -> Self {
        Self {
            start: Instant::now(),
            total: 0,
            per_ip: HashMap::new(),
        }
    }
}
