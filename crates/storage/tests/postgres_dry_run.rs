mod postgres_support;
use opencanvas_storage::*;
use postgres_support::*;
use serde_json::json;

const LOCK: &str =
    "SELECT revision FROM opencanvas_runtime_v1.projects WHERE project_id=$1 FOR UPDATE";

fn rects(count: usize) -> Vec<Operation> {
    (0..count)
        .map(|index| Operation::CreateNode {
            id: format!("rect-{index:03}"),
            kind: NodeKind::Rectangle,
            parent_id: Some("page".into()),
            props: json!({"name": format!("R{index}"), "width": 10, "height": 10}),
            index: None,
        })
        .collect()
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn dry_run_completes_while_a_writer_holds_the_project_row() {
    let (_, storage) = setup();
    let p = project(&storage, "preview-lock");
    p.apply_batch(request(p.id(), 0, "seed", page("Seed")))
        .unwrap();
    let mut control = sql();
    let mut writer = control.transaction().unwrap();
    writer.query_one(LOCK, &[&p.id()]).unwrap();

    // A real commit still waits for the row lock and gives up at lock_timeout.
    let busy = p.apply_batch(request(p.id(), 1, "blocked", rename("Blocked")));
    assert_eq!(busy.unwrap_err().code, "STORAGE_BUSY");

    // The preview reads a snapshot instead, so it never meets lock_timeout.
    let preview = p
        .dry_run_batch(request(p.id(), 1, "preview", rects(400)))
        .unwrap();
    assert!(preview.dry_run);
    assert_eq!(preview.project_id, p.id());
    assert_eq!(preview.base_revision, 1);
    assert!(preview.changed_ids.iter().any(|id| id == "rect-399"));
    assert!(preview.diagnostics.is_empty());
    let stale = p.dry_run_batch(request(p.id(), 0, "stale", rename("Stale")));
    let stale = stale.unwrap_err();
    assert_eq!(stale.code, "REVISION_CONFLICT");
    assert_eq!(stale.current_revision, Some(1));
    writer.rollback().unwrap();

    // Nothing was written or reserved: the same key then commits normally.
    assert_eq!(p.snapshot().unwrap().revision, 1);
    let committed = p
        .apply_batch(request(p.id(), 1, "preview", rects(400)))
        .unwrap();
    assert_eq!(committed.changed_ids, preview.changed_ids);
}

#[test]
#[ignore = "requires fresh PostgreSQL via scripts/task19-check.sh"]
fn dry_run_keeps_request_and_idempotency_checks() {
    let (_, storage) = setup();
    let p = project(&storage, "preview-checks");
    let seed = request(p.id(), 0, "seed", page("Seed"));
    p.apply_batch(seed.clone()).unwrap();

    // A matching key never short-circuits a preview: the stale base conflicts.
    let replayed = p.dry_run_batch(seed).unwrap_err();
    assert_eq!(replayed.code, "REVISION_CONFLICT");
    assert_eq!(replayed.current_revision, Some(1));
    // A reused key with a different canonical request is rejected first.
    let reused = p.dry_run_batch(request(p.id(), 1, "seed", rename("Other")));
    assert_eq!(reused.unwrap_err().code, "IDEMPOTENCY_KEY_REUSED");

    let other = request("p_other", 1, "x", rename("x"));
    assert_eq!(
        p.dry_run_batch(other).unwrap_err().code,
        "VALIDATION_FAILED"
    );
    let bad_key = request(p.id(), 1, "bad key", rename("x"));
    assert_eq!(
        p.dry_run_batch(bad_key).unwrap_err().code,
        "VALIDATION_FAILED"
    );
    let empty = request(p.id(), 1, "empty", vec![]);
    assert_eq!(
        p.dry_run_batch(empty).unwrap_err().code,
        "VALIDATION_FAILED"
    );
    let invalid = vec![Operation::UpdateNode {
        id: "absent".into(),
        props: json!({"name": "x"}),
    }];
    let invalid = p.dry_run_batch(request(p.id(), 1, "invalid", invalid));
    assert_eq!(invalid.unwrap_err().op_index, Some(0));
    assert_eq!(p.commits_after(0, 10).unwrap().len(), 1);
}
