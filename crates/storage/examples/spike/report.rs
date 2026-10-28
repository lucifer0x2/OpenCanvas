use super::fixture::*;
use opencanvas_storage::*;
use rusqlite::{params, Connection};
use serde_json::{json, Value};
use std::{path::Path, time::Instant};

pub fn percentiles(mut x: Vec<u64>) -> Value {
    x.sort();
    let n = x.len();
    if n == 0 {
        return json!({"samples":0});
    }
    json!({"p50":x[(n-1)*50/100],"p95":x[(n-1)*95/100],"max":x[n-1]})
}
pub fn metrics_summary(m: &[BatchMetrics]) -> Value {
    json!({"totalUs":percentiles(m.iter().map(|m|m.total_us).collect()),"queueWaitUs":percentiles(m.iter().map(|m|m.queue_wait_us).collect()),"readUs":percentiles(m.iter().map(|m|m.read_us).collect()),"validateUs":percentiles(m.iter().map(|m|m.validate_us).collect()),"sqlWriteUs":percentiles(m.iter().map(|m|m.sql_write_us).collect()),"commitUs":percentiles(m.iter().map(|m|m.commit_us).collect()),"logicalRequestBytes":m.iter().map(|m|m.request_bytes).sum::<usize>()})
}
pub fn files(root: &Path, slug: &str) -> Value {
    let bytes = |name: &str| std::fs::metadata(root.join(slug).join(name)).map_or(0, |m| m.len());
    json!({"mainBytes":bytes("db.sql"),"walBytes":bytes("db.sql-wal"),"shmBytes":bytes("db.sql-shm")})
}
pub fn checkpoint_and_engine(p: &Project, root: &Path, slug: &str) -> (Value, Value) {
    let reader = Connection::open(root.join(slug).join("db.sql")).unwrap();
    reader.execute_batch("BEGIN").unwrap();
    let pinned: u64 = reader
        .query_row("SELECT current_revision FROM project_meta", [], |r| {
            Ok(r.get::<_, i64>(0)? as u64)
        })
        .unwrap();
    let before = files(root, slug);
    for i in 0..20 {
        let rev = p.snapshot().unwrap().revision;
        p.apply_batch(req(
            p,
            rev,
            format!("checkpoint-{i}"),
            updates(&format!("checkpoint-{i}")),
        ))
        .unwrap();
    }
    let held = p.checkpoint().unwrap();
    let during = files(root, slug);
    assert_eq!(
        reader
            .query_row("SELECT current_revision FROM project_meta", [], |r| Ok(
                r.get::<_, i64>(0)? as u64
            ))
            .unwrap(),
        pinned
    );
    assert!(held.wal_pages > held.checkpointed_pages);
    reader.execute_batch("COMMIT").unwrap();
    let released = p.checkpoint().unwrap();
    assert_eq!(released.wal_pages, released.checkpointed_pages);
    let checkpoint = json!({"pinnedRevision":pinned,"held":held,"released":released,"beforeFiles":before,"heldFiles":during,"releasedFiles":files(root,slug)});
    let mut conn = Connection::open(root.join(format!("engine-{slug}.sql"))).unwrap();
    conn.execute_batch("PRAGMA journal_mode=WAL;PRAGMA synchronous=FULL;PRAGMA fullfsync=ON;CREATE TABLE rows(id INTEGER PRIMARY KEY,value TEXT);").unwrap();
    for i in 0..10 {
        conn.execute("INSERT INTO rows VALUES(?1,'initial')", [i])
            .unwrap();
    }
    let mut writes = Vec::new();
    let mut commits = Vec::new();
    for i in 0..30 {
        let t = Instant::now();
        let tx = conn.transaction().unwrap();
        for row in 0..10 {
            tx.execute(
                "UPDATE rows SET value=?1 WHERE id=?2",
                params![format!("batch-{i}-{}", "x".repeat(128)), row],
            )
            .unwrap();
        }
        writes.push(t.elapsed().as_micros() as u64);
        let t = Instant::now();
        tx.commit().unwrap();
        commits.push(t.elapsed().as_micros() as u64);
    }
    (
        checkpoint,
        json!({"samples":30,"sqlWriteUs":percentiles(writes),"commitUs":percentiles(commits),"note":"10 raw rows, 128-byte values, WAL/FULL/fullfsync; isolates minimal engine overhead, not an equivalent project mutation"}),
    )
}
