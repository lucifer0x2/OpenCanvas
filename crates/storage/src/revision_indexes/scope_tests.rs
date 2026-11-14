//! Lazy read-scope slot: snapshot binding, worst-case note admission, poisoning.
use super::tests::snapshot;
use super::*;

#[test]
fn cached_scope_index_for_another_snapshot_is_neither_used_nor_replaced() {
    let small = Arc::new(snapshot());
    let indexes = Arc::new(RevisionIndexes::new(&small.nodes));
    SharedSnapshot::committed(small.clone(), indexes.clone())
        .trusted_scoped()
        .unwrap()
        .unwrap();
    let first = indexes.read_scope.lock().unwrap().clone().unwrap();
    // Same revision/project, more nodes: the cached positions would be wrong.
    let mut grown = snapshot();
    for i in 0..3 {
        grown.nodes.insert(
            0,
            Node {
                id: format!("frame{i}"),
                parent_id: Some("page".into()),
                sort_key: i,
                kind: NodeKind::Frame,
                props: json!({"width":10,"height":10}),
            },
        );
    }
    let shared = SharedSnapshot::committed(Arc::new(grown), indexes.clone());
    let scoped = shared.trusted_scoped().unwrap().unwrap();
    assert_eq!(scoped.node("page").unwrap().id, "page");
    assert_eq!(scoped.children("page").count(), 3);
    assert_eq!(scoped.frames().count(), 3);
    let cached = indexes.read_scope.lock().unwrap().clone().unwrap();
    assert!(Arc::ptr_eq(&first, &cached));
    assert!(cached.indexes(&small));
}

#[test]
fn five_thousand_notes_on_distinct_128_byte_anchors_fit_the_reservation() {
    let mut snapshot = snapshot();
    let anchor = |i: usize| format!("{i:0>128}");
    for i in 0..5000 {
        snapshot.nodes.push(Node {
            id: anchor(i),
            parent_id: Some("page".into()),
            sort_key: i as i64,
            kind: NodeKind::Rectangle,
            props: json!({"width":1,"height":1}),
        });
    }
    snapshot.notes = (0..5000)
        .map(|i| Note {
            id: format!("note{i:0>124}"),
            node_id: anchor(i),
            text: "Intent".into(),
            author: None,
            resolved: false,
            replies: vec![],
        })
        .collect();
    let snapshot = Arc::new(snapshot);
    let indexes = Arc::new(RevisionIndexes::new(&snapshot.nodes));
    let shared = SharedSnapshot::committed(snapshot.clone(), indexes.clone());
    assert_eq!(
        shared
            .trusted_scoped()
            .unwrap()
            .unwrap()
            .notes_for_nodes([anchor(7).as_str()])
            .len(),
        1
    );
    let retained = indexes.read_scope.lock().unwrap().clone();
    let index = retained.expect("worst-case note graph is retained");
    assert!(index.estimated_bytes() <= indexes.read_weight);
    // Note-position metadata alone stays inside the 2 MiB note reservation.
    let notes: usize = (0..5000).map(|i| 96 + anchor(i).len() + 8).sum();
    assert!(notes <= READ_NOTES_BUDGET, "{notes}");
}

#[test]
fn poisoned_scope_slot_falls_back_to_the_untrusted_path() {
    let snapshot = Arc::new(snapshot());
    let indexes = Arc::new(RevisionIndexes::new(&snapshot.nodes));
    let held = indexes.clone();
    let _ = std::thread::spawn(move || {
        let _guard = held.read_scope.lock().unwrap();
        panic!("poison the lazy scope slot");
    })
    .join();
    assert!(indexes.read_scope.is_poisoned());
    let shared = SharedSnapshot::committed(snapshot, indexes);
    assert!(shared.trusted_scoped().unwrap().is_none());
    // Callers then use their legacy path; `scoped()` fully validates.
    assert_eq!(shared.scoped().unwrap().roots().count(), 1);
}
