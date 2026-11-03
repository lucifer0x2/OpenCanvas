mod common;
mod schemes;
use common::*;
use opencanvas_storage::*;
use serde_json::json;
use std::collections::BTreeSet;

#[test]
fn scoped_style_weight_selects_exact_faces_and_compact_font_closure_uses_placement() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "fixture", schemes::fixture()))
        .unwrap();
    let mut snapshot = p.snapshot().unwrap();
    let bundled = snapshot.fonts[0].clone();
    for (id, weight) in [("regular", 400), ("bold", 700)] {
        let mut font = bundled.clone();
        font.id = id.into();
        font.family = "Scoped Sans".into();
        font.weight = weight;
        snapshot.fonts.push(font);
    }
    snapshot.tokens.push(serde_json::from_value(json!({"id":"weight","name":"Weight","type":"number","value":{"default":400,"themes":{"night":700,"day":400}}})).unwrap());
    snapshot
        .styles
        .iter_mut()
        .find(|s| s.id == "copy")
        .unwrap()
        .value = json!({"fontFamily":"Scoped Sans","fontWeight":{"tokenId":"weight"},"fontSize":14,"lineHeight":20});
    let resolved = resolved_expanded_nodes(&snapshot, "default").unwrap();
    let font = |id: &str| resolved.iter().find(|n| n.id == id).unwrap().props["fontId"].clone();
    assert_eq!(font("one/label"), "bold");
    assert_eq!(font("two/label"), "regular");
    assert_eq!(font("label"), "regular");
    let expanded = instances::expand(&snapshot.nodes).unwrap();
    let one_label = expanded.iter().find(|n| n.id == "one/label").unwrap();
    assert_eq!(
        referenced_font_ids(&snapshot, &[one_label]).unwrap(),
        BTreeSet::from(["bold".into()])
    );
    let labels: Vec<_> = expanded
        .iter()
        .filter(|n| n.kind == NodeKind::Text)
        .collect();
    assert_eq!(
        referenced_font_ids(&snapshot, &labels).unwrap(),
        BTreeSet::from(["regular".into(), "bold".into()])
    );
}
