use super::*;

pub fn concurrency(p: Arc<ProjectHandle>) {
    seed(&p);
    let barrier = Arc::new(std::sync::Barrier::new(3));
    let workers: Vec<_> = [("p1", "r1"), ("p2", "r2")]
        .into_iter()
        .enumerate()
        .map(|(i, (page, node))| {
            let p = p.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                p.apply_scoped_batch(scoped(
                    &p,
                    1,
                    &format!("parallel{i}"),
                    &[page],
                    vec![update(node, json!({"name":page}))],
                ))
            })
        })
        .collect();
    barrier.wait();
    let mut revisions: Vec<_> = workers
        .into_iter()
        .map(|t| t.join().unwrap().unwrap().committed_revision)
        .collect();
    revisions.sort();
    assert_eq!(revisions, [2, 3]);
    assert_eq!(p.snapshot().unwrap().revision, 3);
    let stale = scoped(
        &p,
        1,
        "stale",
        &["p1"],
        vec![update("r1", json!({"name":"stale"}))],
    );
    assert_eq!(
        p.apply_scoped_batch(stale).unwrap_err().code,
        "REVISION_CONFLICT"
    );
    let replay = scoped(
        &p,
        3,
        "replay",
        &["p2"],
        vec![update("r2", json!({"name":"accepted"}))],
    );
    let preview = p.dry_run_scoped_batch(replay.clone()).unwrap();
    assert_eq!(preview.base_revision, 3);
    assert_eq!(p.snapshot().unwrap().revision, 3);
    let result = p.apply_scoped_batch(replay.clone()).unwrap();
    strict(&p, "later", vec![update("r1", json!({"name":"later"}))]);
    let again = p.apply_scoped_batch(replay.clone()).unwrap();
    assert!(again.replayed);
    assert_eq!(again.committed_revision, result.committed_revision);
    let mut changed = replay;
    changed.page_ids = vec!["p1".into(), "p2".into()];
    assert_eq!(
        p.apply_scoped_batch(changed).unwrap_err().code,
        "IDEMPOTENCY_KEY_REUSED"
    );
    assert!(p
        .commits_after(0, 100)
        .unwrap()
        .iter()
        .all(|c| c.operations.is_array() && c.base_revision + 1 == c.revision));
    let base = p.snapshot().unwrap().revision;
    strict(
        &p,
        "delete",
        vec![Operation::DeleteSubtree { id: "r2".into() }],
    );
    strict(
        &p,
        "recreate",
        vec![Operation::CreateNode {
            id: "r2".into(),
            kind: NodeKind::Rectangle,
            parent_id: Some("f2".into()),
            props: json!({"width":20,"height":20}),
            index: None,
        }],
    );
    assert_eq!(
        p.apply_scoped_batch(scoped(
            &p,
            base,
            "aba",
            &["p2"],
            vec![update("r2", json!({"name":"stale"}))]
        ))
        .unwrap_err()
        .code,
        "REVISION_CONFLICT"
    );
}

pub fn restrictions(p: Arc<ProjectHandle>) {
    seed(&p);
    let unsupported = vec![
        vec![token("base", json!("#ffffff"))],
        vec![Operation::ReparentNode {
            id: "r1".into(),
            parent_id: "f2".into(),
            index: None,
        }],
        vec![Operation::DeleteSubtree { id: "f1".into() }],
        vec![Operation::ReorderNode {
            id: "f1".into(),
            index: 0,
        }],
        vec![update("f1", json!({"flowStart":"global"}))],
        vec![Operation::CreateNode {
            id: "newpage".into(),
            kind: NodeKind::Page,
            parent_id: None,
            props: json!({"name":"New"}),
            index: None,
        }],
        vec![Operation::CreateNote {
            id: "note".into(),
            node_id: "r1".into(),
            text: "note".into(),
            author: None,
        }],
    ];
    let before = p.snapshot().unwrap();
    for (i, ops) in unsupported.into_iter().enumerate() {
        let error = p
            .apply_scoped_batch(scoped(&p, 1, &format!("unsupported{i}"), &["p1"], ops))
            .unwrap_err();
        assert_eq!(error.code, "SCOPED_OPERATION_UNSUPPORTED", "{error:?}");
        assert_eq!(p.snapshot().unwrap(), before);
    }
    p.apply_scoped_batch(scoped(
        &p,
        1,
        "move",
        &["p1", "p2"],
        vec![Operation::ReparentNode {
            id: "r1".into(),
            parent_id: "f2".into(),
            index: None,
        }],
    ))
    .unwrap();
    let revision = p.snapshot().unwrap().revision;
    let bad = scoped(
        &p,
        revision,
        "atomic",
        &["p2"],
        vec![
            update("r1", json!({"name":"rollback"})),
            update("missing", json!({"name":"bad"})),
        ],
    );
    assert!(p.apply_scoped_batch(bad).is_err());
    assert_eq!(p.snapshot().unwrap().revision, revision);
    strict(
        &p,
        "note",
        vec![Operation::CreateNote {
            id: "note".into(),
            node_id: "r1".into(),
            text: "note".into(),
            author: None,
        }],
    );
    let revision = p.snapshot().unwrap().revision;
    assert_eq!(
        p.apply_scoped_batch(scoped(
            &p,
            revision,
            "cascade",
            &["p2"],
            vec![Operation::DeleteSubtree { id: "r1".into() }]
        ))
        .unwrap_err()
        .code,
        "SCOPED_OPERATION_UNSUPPORTED"
    );
    let revision = p.snapshot().unwrap().revision;
    strict(
        &p,
        "global-navigation",
        vec![update("f1", json!({"flowStart":"start"}))],
    );
    assert_eq!(
        p.apply_scoped_batch(scoped(
            &p,
            revision,
            "global-stale",
            &["p2"],
            vec![update("r2", json!({"name":"old navigation"}))]
        ))
        .unwrap_err()
        .code,
        "REVISION_CONFLICT"
    );
}
