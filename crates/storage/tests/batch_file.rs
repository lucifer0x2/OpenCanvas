mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};
use std::{fs, path::Path};
mod inbox_guards;

fn file(p: &Project, revision: u64, key: &str, path: &str) -> BatchFileRequest {
    BatchFileRequest {
        project_id: p.id().into(),
        expected_revision: revision,
        idempotency_key: key.into(),
        path: path.into(),
    }
}
fn write(path: &Path, operations: &[Operation]) {
    fs::write(path, serde_json::to_vec(operations).unwrap()).unwrap();
}

#[test]
fn file_commit_content_replay_inline_equivalence_and_restart() {
    let (temp, store, p) = setup();
    let inbox = temp.path().join("fixture/inbox");
    let path = inbox.join("ops.json");
    write(&path, &initial());
    let req = file(&p, 0, "initial", "ops.json");
    let result = p.apply_batch_file(req.clone()).unwrap();
    assert_eq!(result.committed_revision, 1);
    assert!(
        p.apply_batch(request(&p, 0, "initial", initial()))
            .unwrap()
            .replayed
    );
    // Whitespace/key order and filename have no bearing on canonical request hashes.
    let operations: Value = serde_json::from_slice(&fs::read(&path).unwrap()).unwrap();
    fs::write(
        inbox.join("other.json"),
        serde_json::to_string_pretty(&operations).unwrap(),
    )
    .unwrap();
    assert!(
        p.apply_batch_file(file(&p, 0, "initial", "other.json"))
            .unwrap()
            .replayed
    );
    write(
        &path,
        &[Operation::UpdateNode {
            id: "rect".into(),
            props: json!({"width":999}),
        }],
    );
    assert_eq!(
        p.apply_batch_file(req.clone()).unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
    write(&path, &initial());
    let id = p.id().to_owned();
    drop(p);
    drop(store);
    let reopened = Store::open(temp.path()).unwrap();
    let p = reopened.open_project(&id).unwrap();
    assert!(p.apply_batch_file(req).unwrap().replayed);
    assert_eq!(p.snapshot().unwrap().revision, 1);
    assert_eq!(p.commits_after(0, 10).unwrap().len(), 1);
}

#[test]
fn dry_run_full_validation_rollback_and_key_is_never_reserved() {
    let (temp, _store, p) = setup();
    let db = temp.path().join("fixture/db.sql");
    let before = fs::read(&db).unwrap();
    let wal = fs::read(db.with_extension("sql-wal")).unwrap();
    let req = request(&p, 0, "preview", initial());
    let preview = p.dry_run_batch(req.clone()).unwrap();
    assert!(preview.dry_run);
    assert_eq!(preview.base_revision, 0);
    assert!(preview.diagnostics.is_empty());
    assert_eq!(p.snapshot().unwrap().revision, 0);
    assert!(p.commits_after(0, 10).unwrap().is_empty());
    assert_eq!(fs::read(&db).unwrap(), before);
    assert_eq!(fs::read(db.with_extension("sql-wal")).unwrap(), wal);
    let path = temp.path().join("fixture/inbox/ops.json");
    write(&path, &initial());
    let from_file = p
        .dry_run_batch_file(file(&p, 0, "preview", "ops.json"))
        .unwrap();
    assert_eq!(preview, from_file);
    assert_eq!(p.apply_batch(req).unwrap().changed_ids, preview.changed_ids);
    let late_invalid = vec![
        Operation::UpdateNode {
            id: "rect".into(),
            props: json!({"width":999}),
        },
        Operation::UpdateNode {
            id: "text".into(),
            props: json!({"styleId":"missing"}),
        },
    ];
    write(&path, &late_invalid);
    let file_req = file(&p, 1, "invalid", "ops.json");
    let before = p.snapshot().unwrap();
    let error = p.dry_run_batch_file(file_req.clone()).unwrap_err();
    let committed_error = p.apply_batch_file(file_req).unwrap_err();
    assert_eq!(
        serde_json::to_value(error).unwrap(),
        serde_json::to_value(committed_error).unwrap()
    );
    assert_eq!(p.snapshot().unwrap(), before);
    assert_eq!(
        p.dry_run_batch(request(&p, 0, "stale", initial()))
            .unwrap_err()
            .code,
        "REVISION_CONFLICT"
    );
}

#[test]
fn preview_restart_has_no_reserved_key_and_accepts_500_repeated_operations() {
    let (temp, store, p) = setup();
    p.apply_batch(request(&p, 0, "seed", initial())).unwrap();
    let operations = vec![
        Operation::UpdateNode {
            id: "rect".into(),
            props: json!({"width":123}),
        };
        500
    ];
    let req = request(&p, 1, "preview-only", operations);
    let preview = p.dry_run_batch(req.clone()).unwrap();
    assert_eq!(preview.changed_ids, vec!["rect"]);
    assert_eq!(
        p.dry_run_batch(request(&p, 1, "bad key", initial()))
            .unwrap_err()
            .field
            .as_deref(),
        Some("idempotencyKey")
    );
    let too_large = request(
        &p,
        1,
        "bytes",
        vec![Operation::UpdateNode {
            id: "rect".into(),
            props: json!({"name":"x".repeat(MAX_BATCH_BYTES)}),
        }],
    );
    assert_eq!(
        p.dry_run_batch(too_large).unwrap_err().field.as_deref(),
        Some("operations")
    );
    let before = p.snapshot().unwrap();
    let id = p.id().to_owned();
    drop(p);
    drop(store);
    let reopened = Store::open(temp.path()).unwrap();
    let p = reopened.open_project(&id).unwrap();
    assert_eq!(p.snapshot().unwrap(), before);
    let result = p.apply_batch(req).unwrap();
    assert!(!result.replayed);
    assert_eq!(result.committed_revision, 2);
    assert_eq!(result.changed_ids, preview.changed_ids);
    let collision = request(
        &p,
        2,
        "preview-only",
        vec![Operation::UpdateNode {
            id: "rect".into(),
            props: json!({"width":456}),
        }],
    );
    assert_eq!(
        p.dry_run_batch(collision).unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
}
