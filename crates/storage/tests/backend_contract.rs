mod backend_contract_support;
use backend_contract_support::*;
use opencanvas_storage::*;
use serde_json::json;
use std::sync::{Arc, Barrier};

#[test]
fn trait_object_scopes_ids_and_replays_the_original_result_after_later_commits() {
    let (_root, backend, storage) = setup();
    let p = storage.create_project("first", "First").unwrap();
    let other = storage.create_project("second", "Second").unwrap();
    assert_eq!(backend.project_meta("first").unwrap().project_id, p.id());
    assert_eq!(storage.open_project(p.id()).unwrap().id(), p.id());
    assert_eq!(storage.list_projects().unwrap().len(), 2);
    assert_eq!(backend.snapshot("first").unwrap().project_id, p.id());
    let original = request(p.id(), 0, "shared-key", page("Original"));
    let first = backend.apply_batch(p.id(), original.clone()).unwrap();
    other
        .apply_batch(request(other.id(), 0, "shared-key", page("Other")))
        .unwrap();
    p.apply_batch(request(p.id(), 1, "later", rename("Later")))
        .unwrap();
    let replay = p.apply_batch(original.clone()).unwrap();
    assert!(replay.replayed);
    assert_eq!(replay.committed_revision, first.committed_revision);
    assert_eq!(replay.changed_ids, first.changed_ids);
    assert_eq!(replay.render_status, first.render_status);
    assert_eq!(p.snapshot().unwrap().revision, 2);
    assert_eq!(other.snapshot().unwrap().nodes[0].props["name"], "Other");
    let mut collision = original;
    collision.operations = page("Changed request");
    assert_eq!(
        p.apply_batch(collision).unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
    let wrong_scope = request(other.id(), 2, "wrong-scope", rename("Wrong"));
    assert_eq!(
        backend
            .apply_batch(p.id(), wrong_scope)
            .unwrap_err()
            .field
            .as_deref(),
        Some("projectId")
    );
    assert_eq!(p.snapshot().unwrap().nodes[0].props["name"], "Later");
}

#[test]
fn late_failure_and_preview_leave_snapshot_log_and_key_unmodified() {
    let (_root, _backend, storage) = setup();
    let p = storage.create_project("atomic", "Atomic").unwrap();
    p.apply_batch(request(p.id(), 0, "seed", page("Before")))
        .unwrap();
    let before = p.snapshot().unwrap();
    let mut operations = rename("Transient");
    operations.push(Operation::UpdateNode {
        id: "missing".into(),
        props: json!({"name":"bad"}),
    });
    let invalid = request(p.id(), 1, "reusable", operations);
    assert_eq!(p.apply_batch(invalid).unwrap_err().op_index, Some(1));
    assert_eq!(p.snapshot().unwrap(), before);
    assert_eq!(p.commits_after(0, 10).unwrap().len(), 1);
    let valid = request(p.id(), 1, "reusable", rename("After"));
    let preview = p.dry_run_batch(valid.clone()).unwrap();
    assert_eq!(preview.base_revision, 1);
    assert!(preview.dry_run);
    assert_eq!(p.snapshot().unwrap(), before);
    assert_eq!(p.commits_after(0, 10).unwrap().len(), 1);
    let committed = p.apply_batch(valid).unwrap();
    assert!(!committed.replayed);
    assert_eq!(committed.changed_ids, preview.changed_ids);
    assert_eq!(committed.committed_revision, 2);
}

#[test]
fn facade_concurrent_writers_have_one_winner_without_partial_reads() {
    let (_root, _backend, storage) = setup();
    let p = storage.create_project("race", "Race").unwrap();
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
            let p = p.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                let mut operations = rename(&format!("Winner {i}"));
                operations.push(Operation::UpdateNode {
                    id: "peer".into(),
                    props: json!({"name":format!("Winner {i}")}),
                });
                p.apply_batch(request(p.id(), 1, &format!("writer-{i}"), operations))
            })
        })
        .collect();
    barrier.wait();
    for _ in 0..8 {
        let snapshot = p.snapshot().unwrap();
        assert!(matches!(snapshot.revision, 1 | 2));
        assert_eq!(snapshot.nodes.len(), 2);
        assert_eq!(
            snapshot.nodes[0].props["name"],
            snapshot.nodes[1].props["name"]
        );
        if snapshot.revision == 1 {
            assert_eq!(snapshot.nodes[0].props["name"], "Before");
        } else {
            assert!(snapshot.nodes[0].props["name"]
                .as_str()
                .unwrap()
                .starts_with("Winner "));
        }
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
