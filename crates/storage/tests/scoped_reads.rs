mod postgres_support;
mod scoped_support;
use opencanvas_storage::{read_scope::*, *};
use scoped_support::*;
use serde_json::json;
use std::sync::Arc;

fn parity(shared: &SharedSnapshot) -> ScopeStats {
    let scoped = shared
        .trusted_scoped()
        .unwrap()
        .expect("builtin committed source");
    let mut nodes = scoped.expand(ReadScope::Page("p1"), true).unwrap();
    let actual = scoped.resolve(&mut nodes, "night").unwrap();
    let ids: std::collections::BTreeSet<_> = actual.iter().map(|n| &n.id).collect();
    let expected: Vec<_> = resolved_expanded_nodes(&shared.snapshot, "night")
        .unwrap()
        .into_iter()
        .filter(|n| ids.contains(&n.id))
        .collect();
    assert_eq!(actual, expected);
    nodes.stats
}

fn exercise(storage: &Storage, p: &ProjectHandle) {
    seed(p);
    let old = storage.shared_snapshot(p.id()).unwrap();
    let original = parity(&old);
    let again = storage.shared_snapshot(p.id()).unwrap();
    assert_eq!(parity(&again), original);
    assert!(Arc::ptr_eq(&old.indexes, &again.indexes));
    strict(
        p,
        "reorder",
        vec![Operation::ReorderNode {
            id: "instance".into(),
            index: 0,
        }],
    );
    let reordered = storage.shared_snapshot(p.id()).unwrap();
    parity(&reordered);
    // Property-only candidate indexes are built in BTreeMap ID order; the read
    // index must instead bind to the backend's actual sorted snapshot vector.
    strict(p, "sparse", vec![update("r1", json!({"width":21}))]);
    let next = storage.shared_snapshot(p.id()).unwrap();
    parity(&next);
    assert_eq!(parity(&old), original);
    let mut swapped = storage.shared_snapshot(p.id()).unwrap();
    swapped.snapshot = Arc::new((*swapped.snapshot).clone());
    assert!(swapped.trusted_scoped().unwrap().is_none());
    swapped.snapshot = Arc::new({
        let mut invalid = (*swapped.snapshot).clone();
        invalid
            .nodes
            .iter_mut()
            .find(|n| n.id == "r2")
            .unwrap()
            .props["bad"] = json!(true);
        invalid
    });
    assert_eq!(swapped.scoped().err().unwrap().code, "VALIDATION_FAILED");
    let mut swapped_index = storage.shared_snapshot(p.id()).unwrap();
    swapped_index.indexes = old.indexes.clone();
    assert!(swapped_index.trusted_scoped().unwrap().is_none());
}

#[test]
fn sqlite_scope_pairing_warm_commit_order_old_readers_and_field_swap() {
    let (temp, storage, p) = sqlite("scoped-reads");
    exercise(&storage, &p);
    let expected = parity(&storage.shared_snapshot(p.id()).unwrap());
    let project_id = p.id().to_string();
    drop(p);
    drop(storage);
    let fresh: Storage = Store::open(temp.path()).unwrap().into();
    assert_eq!(
        expected,
        parity(&fresh.shared_snapshot(&project_id).unwrap())
    );
}

#[test]
#[ignore = "fresh disposable PostgreSQL via scripts/task19-check.sh"]
fn postgres_scope_pairing_warm_commit_order_old_readers_and_field_swap() {
    let (backend, _) = postgres_support::cached();
    let storage = Storage::new(backend);
    let p = postgres_support::project(&storage, "scoped-reads");
    exercise(&storage, &p);
    let (_, cold) = postgres_support::setup();
    assert_eq!(
        parity(&storage.shared_snapshot(p.id()).unwrap()),
        parity(&cold.shared_snapshot(p.id()).unwrap())
    );
}
