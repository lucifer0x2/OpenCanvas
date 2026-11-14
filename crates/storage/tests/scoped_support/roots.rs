//! A first root frame is global only when it becomes the project's first root.
use super::pages::{frame, page};
use super::*;

pub fn first_roots(p: Arc<ProjectHandle>) {
    // `a` is empty and first; `b` holds the project's first root frame.
    strict(
        &p,
        "seed",
        ops(json!([
            page("a", None),
            page("b", None),
            frame("fb", "b"),
            page("c", None)
        ])),
    );
    let base = revision(&p);
    // `c` follows a page with a root: its first root changes no default.
    p.apply_scoped_batch(scoped(
        &p,
        base,
        "c1",
        &["c"],
        ops(json!([frame("c1", "c")])),
    ))
    .unwrap();
    p.apply_scoped_batch(scoped(
        &p,
        base,
        "b",
        &["b"],
        vec![update("fb", json!({"name":"b"}))],
    ))
    .unwrap();
    // A concurrent first root on the same page conflicts.
    assert_eq!(
        code(
            &p,
            scoped(&p, base, "c2", &["c"], ops(json!([frame("c2", "c")])))
        ),
        "REVISION_CONFLICT"
    );
    // `a` precedes every root: its first root becomes the project default.
    let now = revision(&p);
    let first = ops(json!([frame("fa", "a")]));
    assert_eq!(
        code(&p, scoped(&p, now, "fa", &["a"], first.clone())),
        "SCOPED_OPERATION_UNSUPPORTED"
    );
    strict(&p, "fa", first);
    let stale = scoped(
        &p,
        now,
        "stale",
        &["c"],
        vec![update("c1", json!({"name":"stale"}))],
    );
    assert_eq!(code(&p, stale), "REVISION_CONFLICT");
    // A strict page append keeps other guards valid.
    let now = revision(&p);
    strict(&p, "x", ops(json!([page("x", None)])));
    p.apply_scoped_batch(scoped(
        &p,
        now,
        "after-x",
        &["b"],
        vec![update("fb", json!({"name":"x"}))],
    ))
    .unwrap();
}

pub fn empty_project(p: Arc<ProjectHandle>) {
    let request = scoped(
        &p,
        0,
        "first",
        &["first"],
        ops(json!([page("first", None)])),
    );
    assert_eq!(code(&p, request), "SCOPED_OPERATION_UNSUPPORTED");
    strict(
        &p,
        "first",
        ops(json!([page("first", None), page("second", None)])),
    );
    p.apply_scoped_batch(scoped(
        &p,
        1,
        "third",
        &["third"],
        ops(json!([page("third", None)])),
    ))
    .unwrap();
}
