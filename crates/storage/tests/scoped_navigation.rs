mod scoped_support;
use opencanvas_storage::*;
use scoped_support::*;
use serde_json::json;

fn frame(id: &str) -> Operation {
    Operation::CreateNode {
        id: id.into(),
        kind: NodeKind::Frame,
        parent_id: Some("p1".into()),
        props: json!({"width":40,"height":40}),
        index: None,
    }
}

#[test]
fn safe_root_append_preserves_relative_order_with_numeric_gaps() {
    let (_temp, _storage, p) = sqlite("navigation");
    seed(&p);
    strict(&p, "roots", vec![frame("middle"), frame("tail")]);
    strict(
        &p,
        "gap",
        vec![Operation::DeleteSubtree {
            id: "middle".into(),
        }],
    );
    let before = p.snapshot().unwrap();
    assert_eq!(
        before
            .nodes
            .iter()
            .find(|n| n.id == "tail")
            .unwrap()
            .sort_key,
        2
    );
    let result = p
        .apply_scoped_batch(scoped(&p, 3, "append", &["p1"], vec![frame("append")]))
        .unwrap();
    assert_eq!(result.committed_revision, 4);
    // The append does not invalidate a different page's read at revision 3.
    p.apply_scoped_batch(scoped(
        &p,
        3,
        "other",
        &["p2"],
        vec![update("r2", json!({"name":"independent"}))],
    ))
    .unwrap();
    let revision = p.snapshot().unwrap().revision;
    strict(
        &p,
        "reorder",
        vec![Operation::ReorderNode {
            id: "tail".into(),
            index: 0,
        }],
    );
    assert_eq!(
        p.apply_scoped_batch(scoped(
            &p,
            revision,
            "old-default",
            &["p2"],
            vec![update("r2", json!({"name":"stale"}))]
        ))
        .unwrap_err()
        .code,
        "REVISION_CONFLICT"
    );
}
