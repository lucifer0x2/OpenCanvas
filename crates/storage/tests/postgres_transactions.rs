mod postgres_support;
#[path = "schemes/mod.rs"]
mod schemes;
use postgres_support::*;
use std::{
    collections::BTreeMap,
    sync::{Arc, Barrier},
};

fn versions(id: &str) -> BTreeMap<(String, String), String> {
    sql()
        .query(
            "SELECT kind,id,xmin::text FROM opencanvas_runtime_v1.entities WHERE project_id=$1",
            &[&id],
        )
        .unwrap()
        .into_iter()
        .map(|r| ((r.get(0), r.get(1)), r.get(2)))
        .collect()
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn mutation_writes_only_changed_entity_rows() {
    let (_, storage) = setup();
    let p = project(&storage, "delta");
    p.apply_batch(request(p.id(), 0, "seed", schemes::fixture()))
        .unwrap();
    let before = versions(p.id());
    assert!(before.len() > 10);
    p.apply_batch(request(p.id(), 1, "one-node", rename("Changed")))
        .unwrap();
    let after = versions(p.id());
    assert_eq!(before.len(), after.len());
    for (key, version) in before {
        if key == ("node".into(), "page".into()) {
            assert_ne!(&version, &after[&key]);
        } else {
            assert_eq!(
                &version, &after[&key],
                "unchanged entity rewritten: {key:?}"
            );
        }
    }
    assert_eq!(p.commits_after(0, 10).unwrap().len(), 2);
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn concurrent_identical_retries_replay_after_row_lock_with_one_commit() {
    let (_, storage) = setup();
    let p = project(&storage, "same-key-race");
    let original = request(p.id(), 0, "same", page("One durable commit"));
    let barrier = Arc::new(Barrier::new(3));
    let threads: Vec<_> = (0..2)
        .map(|_| {
            let (_, storage) = setup();
            let p = storage.open_project(p.id()).unwrap();
            let request = original.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                p.apply_batch(request).unwrap()
            })
        })
        .collect();
    barrier.wait();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.replayed).count(), 1);
    let mut a = results[0].clone();
    let mut b = results[1].clone();
    a.replayed = false;
    b.replayed = false;
    assert_eq!(a, b);
    assert_eq!(a.committed_revision, 1);
    assert_eq!(p.snapshot().unwrap().revision, 1);
    assert_eq!(p.commits_after(0, 10).unwrap().len(), 1);
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn deferred_commit_failure_rolls_back_entities_revision_history_and_replay_key() {
    let (_, storage) = setup();
    let p = project(&storage, "commit-fault");
    let before = p.snapshot().unwrap();
    let suffix = uuid::Uuid::new_v4().simple().to_string();
    let function = format!("opencanvas_runtime_v1.reject_{suffix}");
    let trigger = format!("reject_{suffix}");
    // Generated identifiers and canonical UUID only, never user SQL or credentials.
    let ddl = format!("CREATE FUNCTION {function}() RETURNS trigger LANGUAGE plpgsql AS $$ BEGIN RAISE EXCEPTION 'task19 injected commit failure'; END $$; CREATE CONSTRAINT TRIGGER {trigger} AFTER INSERT ON opencanvas_runtime_v1.commits DEFERRABLE INITIALLY DEFERRED FOR EACH ROW WHEN (NEW.project_id='{}') EXECUTE FUNCTION {function}();", p.id());
    sql().batch_execute(&ddl).unwrap();
    let original = request(p.id(), 0, "retry-after-commit-fault", page("Durable"));
    let error = p.apply_batch(original.clone()).unwrap_err();
    assert_eq!(error.code, "STORAGE_FAILED");
    assert!(!error.to_string().contains("task19 injected"));
    assert_eq!(p.snapshot().unwrap(), before);
    assert!(p.commits_after(0, 10).unwrap().is_empty());
    let replay_count: i64 = sql()
        .query_one(
            "SELECT count(*) FROM opencanvas_runtime_v1.idempotency WHERE project_id=$1",
            &[&p.id()],
        )
        .unwrap()
        .get(0);
    assert_eq!(replay_count, 0);
    sql()
        .batch_execute(&format!(
            "DROP TRIGGER {trigger} ON opencanvas_runtime_v1.commits; DROP FUNCTION {function}();"
        ))
        .unwrap();
    let result = p.apply_batch(original).unwrap();
    assert_eq!(result.committed_revision, 1);
    assert!(!result.replayed);
}
