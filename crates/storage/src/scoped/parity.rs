//! Exact old/new closure, sparse footprint and sealed envelope comparisons.
use super::{dependency_index::DependencyIndex, history, index::BatchScope, reference_history};
use crate::*;
#[path = "parity_guard.rs"]
mod guard;

fn seed() -> Snapshot {
    let temp = tempfile::tempdir().unwrap();
    let store = Store::open(temp.path()).unwrap();
    let p = store.create_project("parity", "Parity").unwrap();
    p.apply_batch(BatchRequest { project_id: p.id().into(), expected_revision: 0, idempotency_key: "seed".into(), operations: serde_json::from_value(json!([
        {"op":"upsert_token","token":{"id":"base","name":"Base","type":"color","value":"#112233"}},
        {"op":"upsert_token","token":{"id":"alias","name":"Alias","type":"color","value":{"default":{"aliasId":"base"},"themes":{"night":"#ffffff"}}}},
        {"op":"upsert_style","style":{"id":"fill","name":"Fill","kind":"fill","value":{"tokenId":"alias"}}},
        {"op":"upsert_style","style":{"id":"text-style","name":"Text","kind":"text","value":{"fontId":"font_bundled"}}},
        {"op":"create_node","id":"p1","kind":"page","parentId":null,"props":{"name":"One"}},
        {"op":"create_node","id":"p2","kind":"page","parentId":null,"props":{"name":"Two"}},
        {"op":"create_node","id":"library","kind":"page","parentId":null,"props":{"name":"Library"}},
        {"op":"create_node","id":"empty","kind":"page","parentId":null,"props":{"name":"Empty"}},
        {"op":"create_node","id":"f1","kind":"frame","parentId":"p1","props":{"width":400,"height":400}},
        {"op":"create_node","id":"tail","kind":"frame","parentId":"p1","props":{"width":100,"height":100}},
        {"op":"create_node","id":"f2","kind":"frame","parentId":"p2","props":{"width":400,"height":400}},
        {"op":"create_node","id":"r1","kind":"rectangle","parentId":"f1","props":{"x":0.0,"width":20,"height":20,"fill":{"styleId":"fill"},"links":[{"to":"f2"}]}},
        {"op":"create_node","id":"r2","kind":"rectangle","parentId":"f2","props":{"width":20,"height":20}},
        {"op":"create_node","id":"leaf","kind":"frame","parentId":"library","props":{"component":{"name":"Leaf"},"width":40,"height":40}},
        {"op":"create_node","id":"label","kind":"text","parentId":"leaf","props":{"text":"Hi","width":20,"height":20,"fontSize":12,"lineHeight":16,"styleId":"text-style"}},
        {"op":"create_node","id":"master","kind":"frame","parentId":"library","props":{"component":{"name":"Master"},"width":100,"height":100}},
        {"op":"create_node","id":"nested","kind":"instance","parentId":"master","props":{"componentId":"leaf"}},
        {"op":"create_node","id":"label2","kind":"text","parentId":"master","props":{"text":"Direct","width":20,"height":20,"fontSize":12,"lineHeight":16,"styleId":"text-style","fill":{"styleId":"fill"}}},
        {"op":"create_node","id":"instance","kind":"instance","parentId":"f1","props":{"componentId":"master","overrides":{"label2":{"fill":"#ff0000"}}}},
        {"op":"create_note","id":"note","nodeId":"r2","text":"Anchored","author":"tester"}
    ])).unwrap() }).unwrap();
    p.snapshot().unwrap()
}

fn snapshot(before: &Snapshot, candidate: &Candidate) -> Snapshot {
    let mut after = before.clone();
    after.nodes = candidate.nodes.values().cloned().collect();
    after.tokens = candidate.tokens.values().cloned().collect();
    after.styles = candidate.styles.values().cloned().collect();
    after.fonts = candidate.fonts.values().cloned().collect();
    after.assets = candidate.assets.values().cloned().collect();
    after.notes = candidate.notes.values().cloned().collect();
    after
}

fn closures(snapshot: &Snapshot, index: &DependencyIndex) {
    for pages in [
        vec![],
        vec!["missing".into()],
        vec!["p1".into(), "p1".into()],
        vec!["p1".into()],
        vec!["p2".into()],
        vec!["library".into()],
        vec!["p1".into(), "p2".into()],
    ] {
        let result = index
            .closure(&pages)
            .map_err(|e| serde_json::to_value(e).unwrap());
        let reference = super::dependencies::closure(snapshot, &pages)
            .map_err(|e| serde_json::to_value(e).unwrap());
        assert_eq!(result, reference, "pages={pages:?}");
    }
}

fn step(before: &Snapshot, operations: Vec<Operation>) -> (Snapshot, BTreeSet<String>) {
    let (candidate, changed, mut validation) =
        crate::batch::validated_candidate(before, &operations).unwrap();
    let scope = BatchScope::new(before, &candidate, &changed).unwrap();
    let old = reference_history::footprint(before, &candidate).unwrap();
    assert_eq!(scope.footprint, old, "operations={operations:?}");
    let request = json!({"projectId":before.project_id,"expectedRevision":before.revision,"idempotencyKey":"parity","operations":operations});
    let expected =
        history::envelope_with_scopes(old, request.clone(), request["operations"].clone()).unwrap();
    let actual = history::envelope_with_scopes(
        scope.footprint.clone(),
        request.clone(),
        request["operations"].clone(),
    )
    .unwrap();
    assert_eq!(
        serde_json::to_vec(&actual).unwrap(),
        serde_json::to_vec(&expected).unwrap()
    );
    let after = snapshot(before, &candidate);
    let index =
        DependencyIndex::from_candidate(&candidate, validation.expanded.take(), scope.after, None)
            .unwrap();
    closures(&after, &index);
    (after, scope.footprint)
}

#[test]
fn indexed_closure_and_sparse_history_match_frozen_reference() {
    let before = seed();
    closures(&before, &DependencyIndex::from_snapshot(&before).unwrap());
    let cases = [
        json!([{"op":"update_node","id":"r1","props":{"name":"Changed"}}]),
        json!([{"op":"update_node","id":"r1","props":{"x":-0.0}}]),
        json!([{"op":"update_node","id":"r1","props":{"x":0.0}}]),
        json!([{"op":"update_node","id":"r1","props":{"name":"Temporary"}},{"op":"update_node","id":"r1","props":{"name":"Temporary"}}]),
        json!([{"op":"reparent_node","id":"r1","parentId":"f2"}]),
        json!([{"op":"delete_subtree","id":"f2"}]),
        json!([{"op":"reorder_node","id":"tail","index":0}]),
        json!([{"op":"delete_subtree","id":"tail"}]),
        json!([{"op":"create_node","id":"append","kind":"frame","parentId":"p1","props":{"width":10,"height":10}}]),
        json!([{"op":"create_node","id":"first","kind":"frame","parentId":"empty","props":{"width":10,"height":10}}]),
        json!([{"op":"update_node","id":"f2","props":{"flowStart":"Start"}}]),
        json!([{"op":"upsert_token","token":{"id":"base","name":"Base","type":"color","value":"#aabbcc"}}]),
        json!([{"op":"update_node","id":"instance","props":{"theme":"night"}}]),
        json!([{"op":"update_node","id":"label","props":{"text":"Master changed"}}]),
        json!([{"op":"update_note","id":"note","text":"Edited"}]),
        json!([{"op":"delete_subtree","id":"p2"}]),
        json!([{"op":"reorder_node","id":"p2","index":0}]),
    ];
    for case in cases {
        step(&before, serde_json::from_value(case).unwrap());
    }
    let (_, keys) = step(
        &before,
        serde_json::from_value(json!([{"op":"delete_subtree","id":"f2"}])).unwrap(),
    );
    for expected in ["node:r1", "node:r2", "note:note", "page:p1", "page:p2", "*"] {
        assert!(keys.contains(expected), "{keys:?}");
    }
    let (_, keys) = step(
        &before,
        serde_json::from_value(json!([{"op":"reparent_node","id":"r1","parentId":"f2"}])).unwrap(),
    );
    assert!(keys.contains("page:p1") && keys.contains("page:p2"));
}

#[test]
fn resource_import_full_fallback_preserves_old_and_new_font_families_and_assets() {
    let mut before = seed();
    before
        .fonts
        .iter_mut()
        .find(|f| f.id == "font_bundled")
        .unwrap()
        .asset_id = Some("font-before".into());
    let mut candidate = Candidate::from_snapshot(&before);
    let font = candidate.fonts.get_mut("font_bundled").unwrap();
    font.family = "Replacement".into();
    font.asset_id = Some("font-after".into());
    candidate.assets.insert("font-after".into(), serde_json::from_value(json!({"id":"font-after","contentHash":"bytes-changed","mediaType":"font/ttf","byteSize":1,"relativeKey":"font-key","status":"ready"})).unwrap());
    let new = history::footprint(&before, &candidate).unwrap();
    assert_eq!(
        new,
        reference_history::footprint(&before, &candidate).unwrap()
    );
    assert!(new.contains("font:font_bundled"));
    assert!(new.contains("font-family:Replacement"));
    assert!(new.contains("asset:font-after"));
    assert!(new.contains(&format!(
        "font-family:{}",
        before
            .fonts
            .iter()
            .find(|f| f.id == "font_bundled")
            .unwrap()
            .family
    )));
    let after = snapshot(&before, &candidate);
    closures(&after, &DependencyIndex::from_snapshot(&after).unwrap());
    let index = DependencyIndex::from_snapshot(&after).unwrap();
    assert!(index
        .closure(&["p1".into()])
        .unwrap()
        .contains("asset:font-after"));
}

#[test]
fn absent_validation_expansion_rebuilds_instance_free_and_full_fallback_safely() {
    let mut before = seed();
    before.nodes.retain(|n| {
        !matches!(
            n.id.as_str(),
            "instance" | "nested" | "label" | "label2" | "master" | "leaf"
        )
    });
    Candidate::from_snapshot(&before).validate().unwrap();
    step(
        &before,
        serde_json::from_value(
            json!([{"op":"update_node","id":"r1","props":{"name":"No instances"}}]),
        )
        .unwrap(),
    );
    step(
        &before,
        serde_json::from_value(json!([{"op":"update_node","id":"f1","props":{"theme":"night"}}]))
            .unwrap(),
    );
}
