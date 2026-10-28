#[path = "spike/fixture.rs"]
mod fixture;
#[path = "spike/report.rs"]
mod report;
#[path = "spike/workload.rs"]
mod workload;
use opencanvas_storage::*;
use serde_json::json;
use std::time::Instant;

fn main() {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let started = Instant::now();
    let mut fixtures = Vec::new();
    for size in [100, 5000] {
        let slug = format!("fixture-{size}");
        let p = store.create_project(&slug, &slug).unwrap();
        fixture::seed(&p, size);
        let seed = report::files(temp.path(), &slug);
        let mut sequential = Vec::new();
        for kind in ["create", "update", "reparent"] {
            sequential.push(workload::sequential(&p, kind));
        }
        let mut contention = Vec::new();
        for clients in [1, 4, 8] {
            contention.push(workload::contention(&p, clients));
        }
        let (pin, engine) = report::checkpoint_and_engine(&p, temp.path(), &slug);
        let s = p.snapshot().unwrap();
        assert_eq!(s.nodes.len(), size);
        fixtures.push(json!({"nodes":size,"snapshotBytes":serde_json::to_vec(&s).unwrap().len(),"seedFiles":seed,"finalFiles":report::files(temp.path(),&slug),"sequential":sequential,"contention":contention,"checkpoint":pin,"engineBaseline":engine}));
    }
    println!("{}",serde_json::to_string_pretty(&json!({"sqliteVersion":rusqlite::version(),"elapsedSeconds":started.elapsed().as_secs_f64(),"fixtures":fixtures})).unwrap());
}
