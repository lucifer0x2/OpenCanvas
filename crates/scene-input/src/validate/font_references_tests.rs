use super::*;
use serde_json::{json, Value};

/// The former helper: full Candidate clone, full expansion, mutable theme loop.
fn reference(snapshot: &Snapshot, nodes: &[&Node]) -> BTreeSet<String> {
    let mut candidate = Candidate::from_snapshot(snapshot);
    candidate.nodes = crate::instances::expand(&snapshot.nodes)
        .unwrap()
        .into_iter()
        .map(|n| (n.id.clone(), n))
        .collect();
    let mut ids = BTreeSet::new();
    for theme in candidate.validate_tokens().unwrap() {
        candidate.theme = theme;
        for node in nodes.iter().filter(|n| n.kind == NodeKind::Text) {
            if let Some(id) = candidate
                .resolve_props(node)
                .unwrap()
                .get("fontId")
                .and_then(Value::as_str)
            {
                ids.insert(id.into());
            }
        }
    }
    ids
}

#[test]
fn borrowed_font_context_matches_legacy_themes_family_selection_and_placed_overrides() {
    let snapshot: Snapshot = serde_json::from_value(json!({
        "schemaVersion":4,"projectId":"project","slug":"fonts","name":"Fonts","revision":1,
        "retention":{"oldestCommitRevision":0,"idempotencyPolicy":"lifetime"},
        "tokens":[{"id":"weight","name":"Weight","type":"number","value":{"default":400,"themes":{"night":700}}}],
        "styles":[{"id":"text","name":"Text","kind":"text","value":{"fontFamily":"Family","fontWeight":{"tokenId":"weight"}}},
            {"id":"heavy","name":"Heavy","kind":"text","value":{"fontFamily":"Family","fontWeight":700}}],
        "fonts":[
            {"id":"regular","family":"Family","weight":400,"style":"normal","assetId":"regular-bytes","status":"ready"},
            {"id":"bold","family":"Family","weight":700,"style":"normal","assetId":"bold-bytes","status":"ready"}
        ],"assets":[],"nodes":[
            {"id":"library","kind":"page","parentId":null,"sortKey":0,"props":{}},
            {"id":"page","kind":"page","parentId":null,"sortKey":1,"props":{}},
            {"id":"master","kind":"frame","parentId":"library","sortKey":0,"props":{"width":100,"height":100,"component":{"name":"Text"}}},
            {"id":"label","kind":"text","parentId":"master","sortKey":0,"props":{"text":"Label","styleId":"text","fontSize":12,"lineHeight":16}},
            {"id":"frame","kind":"frame","parentId":"page","sortKey":0,"props":{"width":100,"height":100,"theme":"night"}},
            {"id":"instance","kind":"instance","parentId":"frame","sortKey":0,"props":{"componentId":"master","overrides":{"label":{"styleId":"heavy"}}}},
            {"id":"direct","kind":"text","parentId":"frame","sortKey":1,"props":{"text":"Direct","fontId":"regular","fontWeight":700,"fontSize":12,"lineHeight":16}}
        ]
    })).unwrap();
    let candidate = Candidate::from_snapshot(&snapshot);
    candidate.validate().unwrap();
    let expanded: BTreeMap<_, _> = crate::instances::expand(&snapshot.nodes)
        .unwrap()
        .into_iter()
        .map(|n| (n.id.clone(), n))
        .collect();
    let borrowed = FontReferences::from_candidate(&candidate, &expanded).unwrap();
    let resources_only = FontReferences::from_snapshot(&snapshot, &expanded).unwrap();
    for ids in [
        vec!["label"],
        vec!["instance/label"],
        vec!["direct"],
        vec!["label", "instance/label", "direct"],
    ] {
        let nodes: Vec<_> = ids.iter().map(|id| &expanded[*id]).collect();
        let expected = reference(&snapshot, &nodes);
        assert_eq!(borrowed.for_nodes(&nodes).unwrap(), expected);
        assert_eq!(resources_only.for_nodes(&nodes).unwrap(), expected);
    }
    assert_eq!(
        borrowed.for_nodes(&[&expanded["label"]]).unwrap(),
        BTreeSet::from(["regular".into(), "bold".into()])
    );
    assert_eq!(
        borrowed.for_nodes(&[&expanded["instance/label"]]).unwrap(),
        BTreeSet::from(["bold".into()])
    );
}
