mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::json;
use std::sync::{Arc, Barrier};

#[test]
fn atomic_batch_replay_conflict_and_restart() {
    let (temp, store, p) = setup();
    let req = request(&p, 0, "first", initial());
    let first = p.apply_batch(req.clone()).unwrap();
    assert_eq!(first.committed_revision, 1);
    let retry = p.apply_batch(req.clone()).unwrap();
    assert!(retry.replayed);
    assert_eq!(retry.committed_revision, 1);
    let mut changed = req.clone();
    changed.operations.push(Operation::UpdateNode {
        id: "rect".into(),
        props: json!({"width":200}),
    });
    assert_eq!(
        p.apply_batch(changed).unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
    let stale = request(&p, 0, "stale", initial());
    assert_eq!(p.apply_batch(stale).unwrap_err().current_revision, Some(1));
    let invalid = request(
        &p,
        1,
        "invalid",
        vec![
            Operation::UpdateNode {
                id: "rect".into(),
                props: json!({"width":999}),
            },
            create(
                "bad",
                NodeKind::Image,
                Some("frame"),
                json!({"assetId":"missing"}),
            ),
        ],
    );
    let e = p.apply_batch(invalid).unwrap_err();
    assert_eq!(e.code, "VALIDATION_FAILED");
    assert_eq!(e.op_index, Some(1));
    let before = p.snapshot().unwrap();
    assert_eq!(before.revision, 1);
    assert_eq!(before.nodes.len(), 4);
    assert_eq!(
        before.nodes.iter().find(|n| n.id == "rect").unwrap().props["width"],
        100
    );
    assert_eq!(p.commits_after(0, 100).unwrap().len(), 1);
    let id = p.id().to_string();
    drop(p);
    drop(store);
    let reopened = Store::open(temp.path()).unwrap();
    let p = reopened.open_project(&id).unwrap();
    assert_eq!(p.snapshot().unwrap(), before);
    assert!(p.apply_batch(req).unwrap().replayed);
}

#[test]
fn ordered_tree_reparent_reorder_delete_and_cycle() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let ops = vec![
        create(
            "group",
            NodeKind::Group,
            Some("frame"),
            json!({"width":300,"height":200}),
        ),
        Operation::ReparentNode {
            id: "rect".into(),
            parent_id: "group".into(),
            index: Some(0),
        },
        Operation::ReorderNode {
            id: "group".into(),
            index: 0,
        },
    ];
    p.apply_batch(request(&p, 1, "move", ops)).unwrap();
    let s = p.snapshot().unwrap();
    assert_eq!(
        s.nodes
            .iter()
            .find(|n| n.id == "rect")
            .unwrap()
            .parent_id
            .as_deref(),
        Some("group")
    );
    let cycle = vec![Operation::ReparentNode {
        id: "frame".into(),
        parent_id: "group".into(),
        index: None,
    }];
    assert_eq!(
        p.apply_batch(request(&p, 2, "cycle", cycle))
            .unwrap_err()
            .code,
        "VALIDATION_FAILED"
    );
    p.apply_batch(request(
        &p,
        2,
        "delete",
        vec![Operation::DeleteSubtree { id: "group".into() }],
    ))
    .unwrap();
    assert_eq!(p.snapshot().unwrap().nodes.len(), 3);
}

#[test]
fn one_writer_wins_same_revision_and_reads_never_see_half_batch() {
    let (_temp, store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    assert!(Arc::ptr_eq(&p, &store.open_project("fixture").unwrap()));
    let barrier = Arc::new(Barrier::new(3));
    let threads: Vec<_> = (0..2)
        .map(|i| {
            let p = p.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                p.apply_batch(request(
                    &p,
                    1,
                    &format!("writer-{i}"),
                    vec![
                        Operation::UpdateNode {
                            id: "rect".into(),
                            props: json!({"width":200+i}),
                        },
                        Operation::UpdateNode {
                            id: "text".into(),
                            props: json!({"name":format!("writer-{i}")}),
                        },
                    ],
                ))
            })
        })
        .collect();
    barrier.wait();
    for _ in 0..10 {
        let s = p.snapshot().unwrap();
        let r = s.nodes.iter().find(|n| n.id == "rect").unwrap();
        let t = s.nodes.iter().find(|n| n.id == "text").unwrap();
        if s.revision == 1 {
            assert_eq!(r.props["width"], 100);
            assert!(t.props.get("name").is_none());
        } else {
            assert_eq!(s.revision, 2);
            let w = r.props["width"].as_u64().unwrap() - 200;
            assert_eq!(t.props["name"], format!("writer-{w}"));
        }
    }
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| r.as_ref().is_err_and(|e| e.code == "REVISION_CONFLICT"))
            .count(),
        1
    );
}

#[test]
fn runtime_pragmas_limits_and_safe_errors() {
    let (temp, _store, p) = setup();
    assert!(rusqlite::version_number() >= 3_051_003);
    let conn = rusqlite::Connection::open(temp.path().join("fixture/db.sql")).unwrap();
    assert_eq!(
        conn.query_row("PRAGMA journal_mode", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
    assert_eq!(
        p.apply_batch(request(&p, 0, "empty", vec![]))
            .unwrap_err()
            .code,
        "VALIDATION_FAILED"
    );
    let mut req = request(&p, u64::MAX, "bad-revision", initial());
    assert_eq!(
        p.apply_batch(req.clone()).unwrap_err().code,
        "REVISION_CONFLICT"
    );
    req.project_id = "wrong".into();
    let e = p.apply_batch(req).unwrap_err();
    assert!(!serde_json::to_string(&e)
        .unwrap()
        .contains(temp.path().to_str().unwrap()));
    assert_eq!(
        p.commits_after(u64::MAX, 10).unwrap_err().code,
        "RESET_REQUIRED"
    );
}
