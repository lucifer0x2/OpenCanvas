//! Scoped note writes: anchors on guarded pages, conflicts through anchor pages.
use super::*;

pub fn notes(p: Arc<ProjectHandle>) {
    seed(&p);
    let note = |id: &str, node: &str| json!({"op":"create_note","id":id,"nodeId":node,"text":id});
    strict(
        &p,
        "notes",
        ops(json!([note("n1", "r1"), note("n2", "r2")])),
    );
    let base = revision(&p);
    let context = edit_context(p.snapshot().unwrap(), vec!["p1".into()]).unwrap();
    let keys = context["dependencies"].as_array().unwrap();
    assert!(keys.contains(&json!("note:n1")) && !keys.contains(&json!("note:n2")));

    // Unrelated: note writes on p1 and p2 from the same stale read both commit.
    p.apply_scoped_batch(scoped(
        &p,
        base,
        "a",
        &["p1"],
        ops(json!([note("a", "r1"),
        {"op":"update_note","id":"n1","text":"edited","resolved":true}])),
    ))
    .unwrap();
    p.apply_scoped_batch(scoped(
        &p,
        base,
        "b",
        &["p2"],
        ops(json!([
        {"op":"reply_note","id":"n2","text":"reply"}])),
    ))
    .unwrap();
    p.apply_scoped_batch(scoped(
        &p,
        base,
        "c",
        &["library"],
        vec![update("leaf_box", json!({"name":"c"}))],
    ))
    .unwrap();
    // Related: one note-only write on p1 makes every older p1 read stale.
    let base = revision(&p);
    let resolve = ops(json!([{"op":"update_note","id":"n1","resolved":false}]));
    p.apply_scoped_batch(scoped(&p, base, "a2", &["p1"], resolve))
        .unwrap();
    for (key, operations) in [
        ("d", vec![update("r1", json!({"name":"stale"}))]),
        ("e", ops(json!([{"op":"delete_note","id":"n1"}]))),
        ("f", ops(json!([note("f", "r1")]))),
    ] {
        assert_eq!(
            code(&p, scoped(&p, base, key, &["p1"], operations)),
            "REVISION_CONFLICT"
        );
    }

    // Current and new anchors must be guarded.
    let now = revision(&p);
    for (key, operations) in [
        ("g", ops(json!([{"op":"update_note","id":"n2","text":"x"}]))),
        (
            "h",
            ops(json!([{"op":"update_note","id":"n1","nodeId":"r2"}])),
        ),
        ("i", ops(json!([note("i", "r2")]))),
        ("j", ops(json!([{"op":"delete_note","id":"n2"}]))),
    ] {
        assert_eq!(
            code(&p, scoped(&p, now, key, &["p1"], operations)),
            "SCOPED_OPERATION_UNSUPPORTED"
        );
    }
    assert_eq!(revision(&p), now);
    let move_note = ops(json!([{"op":"update_note","id":"n1","nodeId":"r2"}]));
    p.apply_scoped_batch(scoped(&p, now, "move", &["p1", "p2"], move_note))
        .unwrap();
    // The move landed a note on p2: a p2 read from before it is stale.
    let stale = scoped(
        &p,
        now,
        "k",
        &["p2"],
        vec![update("r2", json!({"name":"k"}))],
    );
    assert_eq!(code(&p, stale), "REVISION_CONFLICT");

    // Delete/recreate of the same note ID between read and write is detected.
    let base = revision(&p);
    strict(&p, "del", ops(json!([{"op":"delete_note","id":"n2"}])));
    strict(&p, "re", ops(json!([note("n2", "r2")])));
    let reply = ops(json!([{"op":"reply_note","id":"n2","text":"aba"}]));
    assert_eq!(
        code(&p, scoped(&p, base, "aba", &["p2"], reply)),
        "REVISION_CONFLICT"
    );

    // A cascade deletes notes on the guarded page; a note attached after the
    // read makes the delete stale.
    let base = revision(&p);
    let rect = json!({"op":"create_node","id":"r3","kind":"rectangle","parentId":"f1",
        "props":{"width":5,"height":5}});
    strict(&p, "r3", ops(json!([rect])));
    let after_create = revision(&p);
    p.apply_scoped_batch(scoped(
        &p,
        after_create,
        "late",
        &["p1"],
        ops(json!([note("late", "r3")])),
    ))
    .unwrap();
    let delete = vec![Operation::DeleteSubtree { id: "r3".into() }];
    assert_eq!(
        code(
            &p,
            scoped(&p, after_create, "stale-cascade", &["p1"], delete.clone())
        ),
        "REVISION_CONFLICT"
    );
    let unrelated = scoped(
        &p,
        base,
        "p2-ok",
        &["p2"],
        vec![update("r2", json!({"name":"ok"}))],
    );
    p.apply_scoped_batch(unrelated).unwrap();
    p.apply_scoped_batch(scoped(&p, revision(&p), "cascade", &["p1"], delete))
        .unwrap();
    assert!(p.snapshot().unwrap().notes.iter().all(|n| n.id != "late"));
}
