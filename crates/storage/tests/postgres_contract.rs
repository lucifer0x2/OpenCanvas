mod postgres_support;
use opencanvas_storage::*;
use postgres_support::*;
use serde_json::json;
use std::sync::{Arc, Barrier};

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn real_postgres_replay_scope_reconnect_atomic_preview_and_ordered_history() {
    let (backend, storage) = setup();
    let p = project(&storage, "contract");
    let other = project(&storage, "scope");
    let meta = backend.project_meta(p.id()).unwrap();
    assert_eq!(storage.open_project(&meta.slug).unwrap().id(), p.id());
    assert!(storage.list_projects().unwrap().contains(&meta));
    let original = request(p.id(), 0, "shared", page("Before"));
    let first = p.apply_batch(original.clone()).unwrap();
    other
        .apply_batch(request(other.id(), 0, "shared", page("Other")))
        .unwrap();
    let before = p.snapshot().unwrap();
    let mut invalid = rename("Must rollback");
    invalid.push(Operation::UpdateNode {
        merge: false,
        id: "missing".into(),
        props: json!({"name":"bad"}),
    });
    let e = p
        .apply_batch(request(p.id(), 1, "reusable", invalid))
        .unwrap_err();
    assert_eq!(e.op_index, Some(1));
    assert_eq!(p.snapshot().unwrap(), before);
    assert_eq!(p.commits_after(0, 10).unwrap().len(), 1);
    let next = request(p.id(), 1, "reusable", rename("After"));
    let preview = p.dry_run_batch(next.clone()).unwrap();
    assert!(preview.dry_run);
    assert_eq!(p.snapshot().unwrap(), before);
    let committed = p.apply_batch(next).unwrap();
    assert_eq!(committed.changed_ids, preview.changed_ids);
    let (_, reopened) = setup();
    let reopened = reopened.open_project(p.id()).unwrap();
    let replay = reopened.apply_batch(original.clone()).unwrap();
    let mut expected = first;
    expected.replayed = true;
    assert_eq!(replay, expected);
    let mut collision = original;
    collision.operations = page("Changed");
    assert_eq!(
        reopened.apply_batch(collision).unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
    let bad_scope = request(other.id(), 2, "scope", rename("Wrong"));
    assert_eq!(
        p.apply_batch(bad_scope).unwrap_err().field.as_deref(),
        Some("projectId")
    );
    let commits = reopened.commits_after(0, 10).unwrap();
    assert_eq!(
        commits.iter().map(|c| c.revision).collect::<Vec<_>>(),
        [1, 2]
    );
    assert_eq!(commits[0].base_revision, 0);
    assert_eq!(commits[1].base_revision, 1);
    assert_eq!(commits[1].operations[0]["props"]["name"], "After");
    assert_eq!(reopened.commits_after(0, 1).unwrap().len(), 1);
    assert!(reopened.commits_after(2, 10).unwrap().is_empty());
    let ahead = reopened.commits_after(99, 10).unwrap_err();
    assert_eq!(ahead.code, "RESET_REQUIRED");
    assert_eq!(ahead.current_revision, Some(2));
    assert_eq!(other.snapshot().unwrap().nodes[0].props["name"], "Other");
    sql()
        .execute(
            "UPDATE opencanvas_runtime_v1.projects SET oldest_revision=2 WHERE project_id=$1",
            &[&p.id()],
        )
        .unwrap();
    let expired = reopened.commits_after(0, 10).unwrap_err();
    assert_eq!(expired.code, "RESET_REQUIRED");
    assert_eq!(expired.current_revision, Some(2));
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn two_independent_backends_serialize_writers_and_snapshots_never_mix_commits() {
    let (_, storage) = setup();
    let p = project(&storage, "race");
    let mut seed = page("Before");
    seed.push(Operation::CreateNode {
        id: "peer".into(),
        kind: NodeKind::Page,
        parent_id: None,
        props: json!({"name":"Before"}),
        index: None,
    });
    p.apply_batch(request(p.id(), 0, "seed", seed)).unwrap();
    let barrier = Arc::new(Barrier::new(3));
    let threads: Vec<_> = (0..2)
        .map(|i| {
            let (_, independent) = setup();
            let p = independent.open_project(p.id()).unwrap();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let mut ops = rename(&format!("Winner {i}"));
                ops.push(Operation::UpdateNode {
                    merge: false,
                    id: "peer".into(),
                    props: json!({"name":format!("Winner {i}")}),
                });
                p.apply_batch(request(p.id(), 1, &format!("writer-{i}"), ops))
            })
        })
        .collect();
    barrier.wait();
    for _ in 0..12 {
        let s = p.snapshot().unwrap();
        assert!(matches!(s.revision, 1 | 2));
        assert_eq!(s.nodes[0].props["name"], s.nodes[1].props["name"]);
        assert_eq!(s.revision == 1, s.nodes[0].props["name"] == "Before");
    }
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| r
                .as_ref()
                .is_err_and(|e| e.code == "REVISION_CONFLICT" && e.current_revision == Some(2)))
            .count(),
        1
    );
    assert_eq!(p.commits_after(0, 10).unwrap().len(), 2);
}
