mod postgres_support;
mod scoped_support;
use opencanvas_storage::{read_scope::*, *};
use scoped_support::*;
use serde_json::json;
use std::sync::Arc;

/// Shared with scene-input: the independent full-expansion closure reference.
#[path = "../../scene-input/tests/scoped_reference/mod.rs"]
mod scoped_reference;

const SCOPES: [(ReadScope<'static>, bool); 6] = [
    (ReadScope::Page("p1"), true),
    (ReadScope::Page("p1"), false),
    (ReadScope::Target("f1"), false),
    (ReadScope::Target("instance"), true),
    (ReadScope::Target("instance/nested/leaf_box"), false),
    (ReadScope::Page("library"), false),
];

fn parity(shared: &SharedSnapshot) -> Vec<ScopeStats> {
    let scoped = shared
        .trusted_scoped()
        .unwrap()
        .expect("builtin committed source");
    let all = resolved_expanded_nodes(&shared.snapshot, "night").unwrap();
    SCOPES
        .into_iter()
        .map(|(scope, definitions)| {
            let mut nodes = scoped.expand(scope, definitions).unwrap();
            let actual = scoped.resolve(&mut nodes, "night").unwrap();
            let ids: Vec<_> = actual.iter().map(|n| n.id.clone()).collect();
            let reference = scoped_reference::reference_ids(&shared.snapshot, scope, definitions);
            assert_eq!(ids, reference, "{scope:?} definitions={definitions}");
            let expected: Vec<_> = all
                .iter()
                .filter(|n| ids.contains(&n.id))
                .cloned()
                .collect();
            assert_eq!(actual, expected);
            nodes.stats
        })
        .collect()
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
