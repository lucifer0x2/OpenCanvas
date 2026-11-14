use super::*;
use serde_json::json;

fn files(sizes: &[usize]) -> Vec<StoredFile> {
    sizes
        .iter()
        .map(|&n| StoredFile {
            meta: json!({"bytes":n}),
            bytes: vec![7; n],
        })
        .collect()
}

fn store(ttl: u64, max_bytes: usize) -> ExportStore {
    ExportStore::new(ExportRetention {
        ttl: Duration::from_secs(ttl),
        max_bytes,
    })
}

fn live(store: &ExportStore, id: &str) -> usize {
    match store.lookup(id) {
        Lookup::Live(files) => files.len(),
        Lookup::Expired => panic!("{id} expired"),
        Lookup::Unknown => panic!("{id} unknown"),
    }
}

#[test]
fn files_live_until_ttl_then_answer_expired_with_their_project() {
    let mut store = store(60, 1024);
    let start = Instant::now();
    let (expires_at, evicted) = store.insert("a", "p1", files(&[3, 4]), start);
    assert!(evicted.is_empty());
    let unix = SystemTime::now().duration_since(UNIX_EPOCH).unwrap();
    assert!((expires_at as i64 - unix.as_secs() as i64 - 60).abs() <= 1);
    assert!(store.purge(start + Duration::from_secs(59)).is_empty());
    assert_eq!(live(&store, "a"), 2);
    assert_eq!(store.purge(start + Duration::from_secs(60)), ["a"]);
    assert!(matches!(store.lookup("a"), Lookup::Expired));
    assert_eq!(store.project("a"), Some("p1"));
    assert!(matches!(store.lookup("never"), Lookup::Unknown));
    assert_eq!(store.project("never"), None);
    assert_eq!(store.bytes, 0);
}

#[test]
fn byte_budget_evicts_oldest_jobs_first_and_always_keeps_the_newest() {
    let mut store = store(60, 100);
    let now = Instant::now();
    store.insert("a", "p", files(&[40]), now);
    store.insert("b", "p", files(&[30, 10]), now);
    let (_, evicted) = store.insert("c", "p", files(&[50]), now);
    assert_eq!(evicted, ["a"]);
    assert_eq!((live(&store, "b"), live(&store, "c")), (2, 1));
    assert!(matches!(store.lookup("a"), Lookup::Expired));
    let (_, evicted) = store.insert("huge", "p", files(&[150]), now);
    assert_eq!(evicted, ["b", "c"]);
    assert_eq!(live(&store, "huge"), 1);
    assert_eq!(store.bytes, 150);
}

#[test]
fn many_jobs_beyond_the_record_cap_keep_their_files() {
    let mut store = store(60, 1 << 20);
    let now = Instant::now();
    for i in 0..super::super::retain::RECORD_CAP * 3 {
        store.insert(&format!("job{i}"), "p", files(&[16]), now);
    }
    assert_eq!(live(&store, "job0"), 1);
}

#[test]
fn expired_memory_is_bounded() {
    let mut store = store(0, 1 << 20);
    let now = Instant::now();
    for i in 0..GONE_CAP + 5 {
        store.insert(&format!("job{i}"), "p", files(&[1]), now);
    }
    store.purge(now);
    assert_eq!(store.gone.len(), GONE_CAP);
    assert!(store.live.is_empty());
    assert!(matches!(store.lookup("job0"), Lookup::Unknown));
    assert!(matches!(store.lookup("job6"), Lookup::Expired));
}
