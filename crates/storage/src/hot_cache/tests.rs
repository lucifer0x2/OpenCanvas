use super::*;
use crate::{Retention, SCHEMA_VERSION};
use std::{sync::Barrier, thread, time::Instant};

fn snapshot(id: &str, revision: u64) -> Arc<Snapshot> {
    Arc::new(Snapshot {
        schema_version: SCHEMA_VERSION,
        project_id: id.into(),
        slug: id.into(),
        name: "Snapshot".into(),
        revision,
        nodes: vec![],
        tokens: vec![],
        styles: vec![],
        assets: vec![],
        fonts: vec![],
        notes: vec![],
        retention: Retention {
            oldest_commit_revision: 1,
            idempotency_policy: "retained".into(),
        },
    })
}

#[test]
fn hits_share_the_resident_snapshot_without_copying_it() {
    let cache = SnapshotCache::new("db");
    let filled = snapshot("project", 3);
    cache.put(&filled, 1);
    let first = cache.get("project", 1, 3).unwrap();
    let second = cache.get("project", 1, 3).unwrap();
    assert!(Arc::ptr_eq(&first, &filled));
    assert!(Arc::ptr_eq(&first, &second));
    assert!(cache.get("project", 2, 3).is_none());
    assert!(cache.get("project", 1, 4).is_none());
}

#[test]
fn fixed_expiration_uses_insertion_time_and_duplicate_fills_do_not_renew_it() {
    let production = SnapshotCache::new("db");
    assert_eq!(
        production.entries.policy().time_to_live(),
        Some(Duration::from_secs(3_600))
    );
    assert_eq!(production.entries.policy().time_to_idle(), None);
    let ttl = Duration::from_millis(400);
    let cache = SnapshotCache::with_limits("db".into(), ttl, MAX_WEIGHT, MAX_ENTRY_WEIGHT);
    let snapshot = snapshot("project", 1);
    cache.put(&snapshot, 1);
    let inserted = Instant::now();
    thread::sleep(ttl / 2);
    assert_eq!(cache.get("project", 1, 1), Some(snapshot.clone()));
    cache.put(&snapshot, 1);
    thread::sleep((ttl + Duration::from_millis(30)).saturating_sub(inserted.elapsed()));
    assert!(cache.get("project", 1, 1).is_none());
    cache.put(&snapshot, 1);
    assert_eq!(cache.get("project", 1, 1), Some(snapshot));
}

#[test]
fn concurrent_fills_and_recreation_keep_only_the_newest_resident_identity() {
    let cache = SnapshotCache::new("db");
    let old = snapshot("project", u64::MAX - 1);
    let newest = snapshot("project", u64::MAX);
    let barrier = Barrier::new(3);
    thread::scope(|scope| {
        for snapshot in [&old, &newest] {
            let cache = cache.clone();
            let barrier = &barrier;
            scope.spawn(move || {
                barrier.wait();
                cache.put(snapshot, 9);
            });
        }
        barrier.wait();
    });
    assert_eq!(cache.get("project", 9, u64::MAX), Some(newest.clone()));
    assert!(cache.get("project", 9, u64::MAX - 1).is_none());
    let replacement = snapshot("project", 0);
    cache.put(&replacement, 10);
    cache.put(&newest, 9);
    assert_eq!(cache.get("project", 10, 0), Some(replacement));
    assert!(cache.get("project", 9, u64::MAX).is_none());
    cache.entries.run_pending_tasks();
    assert_eq!(cache.entries.entry_count(), 1);
}

#[test]
fn admission_limit_and_weighted_capacity_bound_large_snapshots() {
    let sample = snapshot("first", 1);
    let bytes = serde_json::to_vec(&*sample).unwrap().len() as u32;
    assert_eq!(snapshot_weight(&sample, bytes), Some(bytes));
    assert_eq!(snapshot_weight(&sample, bytes - 1), None);
    let cache = SnapshotCache::with_limits("db".into(), TTL, u64::from(bytes) * 2, bytes);
    cache.put(&sample, 1);
    cache.put(&snapshot("other", 1), 1);
    cache.entries.run_pending_tasks();
    assert_eq!(cache.entries.entry_count(), 2);
    cache.put(&snapshot("third", 1), 1);
    cache.entries.run_pending_tasks();
    assert!(cache.entries.weighted_size() <= u64::from(bytes) * 2);
    let mut oversized = Arc::unwrap_or_clone(sample);
    oversized.revision = 2;
    oversized.name.push('!');
    cache.put(&Arc::new(oversized), 1);
    assert!(cache.get("first", 1, 2).is_none());
}
