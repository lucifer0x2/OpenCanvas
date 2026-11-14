//! Page appends stay local unless they move the default page or reorder pages.
use super::*;

pub fn page(id: &str, index: Option<usize>) -> Value {
    json!({"op":"create_node","id":id,"kind":"page","parentId":null,"props":{"name":id},"index":index})
}

pub fn frame(id: &str, parent: &str) -> Value {
    json!({"op":"create_node","id":id,"kind":"frame","parentId":parent,"props":{"width":40,"height":40}})
}

pub fn pages(p: Arc<ProjectHandle>) {
    seed(&p);
    // Gap in page sort keys, so the next append compacts `library`.
    strict(
        &p,
        "gap",
        vec![Operation::DeleteSubtree { id: "p2".into() }],
    );
    let base = revision(&p);
    let created = ops(json!([page("new", None), frame("nf", "new"),
        {"op":"create_node","id":"nr","kind":"rectangle","parentId":"nf","props":{"width":5,"height":5}}]));
    p.apply_scoped_batch(scoped(&p, base, "new", &["new"], created))
        .unwrap();
    let snapshot = p.snapshot().unwrap();
    let order: Vec<_> = snapshot
        .nodes
        .iter()
        .filter(|n| n.kind == NodeKind::Page)
        .map(|n| (n.sort_key, n.id.as_str()))
        .collect();
    assert_eq!(order, [(0, "p1"), (1, "library"), (2, "new")]);
    // Unrelated stale guards, including the compacted page, still commit.
    p.apply_scoped_batch(scoped(
        &p,
        base,
        "p1",
        &["p1"],
        vec![update("r1", json!({"name":"a"}))],
    ))
    .unwrap();
    p.apply_scoped_batch(scoped(
        &p,
        base,
        "lib",
        &["library"],
        vec![update("leaf_box", json!({"name":"b"}))],
    ))
    .unwrap();
    // A second session's append from the same stale read lands after it.
    p.apply_scoped_batch(scoped(
        &p,
        base,
        "new2",
        &["new2"],
        ops(json!([page("new2", None)])),
    ))
    .unwrap();
    // Strict appends no longer invalidate unrelated guards either.
    let base = revision(&p);
    strict(
        &p,
        "strict-page",
        ops(json!([page("s", None), frame("sf", "s")])),
    );
    p.apply_scoped_batch(scoped(
        &p,
        base,
        "after-strict",
        &["p1"],
        vec![update("r1", json!({"name":"c"}))],
    ))
    .unwrap();

    // Related: the new page's dependencies changed since the read.
    let base = revision(&p);
    strict(
        &p,
        "master",
        vec![update("leaf_box", json!({"name":"changed"}))],
    );
    let instance = ops(json!([page("uses", None), frame("uf", "uses"),
        {"op":"create_node","id":"ui","kind":"instance","parentId":"uf","props":{"componentId":"master"}}]));
    assert_eq!(
        code(&p, scoped(&p, base, "uses", &["uses"], instance)),
        "REVISION_CONFLICT"
    );
    // Related: the new page ID was created and deleted meanwhile (page deletion is global).
    let base = revision(&p);
    strict(&p, "aba-create", ops(json!([page("aba", None)])));
    strict(
        &p,
        "aba-delete",
        vec![Operation::DeleteSubtree { id: "aba".into() }],
    );
    assert_eq!(
        code(
            &p,
            scoped(&p, base, "aba", &["aba"], ops(json!([page("aba", None)])))
        ),
        "REVISION_CONFLICT"
    );

    // Still strict: page insert before others, page rename/delete, unguarded new page.
    let now = revision(&p);
    for (key, pages, operations) in [
        (
            "insert",
            vec!["first"],
            ops(json!([page("first", Some(0))])),
        ),
        (
            "rename",
            vec!["p1"],
            vec![update("p1", json!({"name":"Renamed"}))],
        ),
        (
            "delete",
            vec!["s"],
            vec![Operation::DeleteSubtree { id: "s".into() }],
        ),
        ("unguarded", vec!["p1"], ops(json!([page("x", None)]))),
    ] {
        assert_eq!(
            code(&p, scoped(&p, now, key, &pages, operations)),
            "SCOPED_OPERATION_UNSUPPORTED",
            "{key}"
        );
    }
    assert_eq!(revision(&p), now);
    strict(&p, "strict-insert", ops(json!([page("first", Some(0))])));
    let stale = scoped(
        &p,
        now,
        "stale",
        &["p1"],
        vec![update("r1", json!({"name":"stale"}))],
    );
    assert_eq!(code(&p, stale), "REVISION_CONFLICT");
}
