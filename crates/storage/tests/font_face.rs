mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};

fn fixture() -> Snapshot {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let mut snapshot = p.snapshot().unwrap();
    for (id, weight, style) in [
        ("f400", 400, "normal"),
        ("f600", 600, "normal"),
        ("f700i", 700, "italic"),
    ] {
        let mut font = snapshot.fonts[0].clone();
        font.id = id.into();
        font.family = "Test Sans".into();
        font.weight = weight;
        font.style = style.into();
        snapshot.fonts.push(font);
    }
    snapshot
}
fn props(snapshot: &mut Snapshot, authored: Value) {
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "text")
        .unwrap()
        .props = json!({"text":"Face","fontSize":16,"lineHeight":24});
    snapshot
        .nodes
        .iter_mut()
        .find(|n| n.id == "text")
        .unwrap()
        .props
        .as_object_mut()
        .unwrap()
        .extend(authored.as_object().unwrap().clone());
}
fn resolved(snapshot: &Snapshot) -> Value {
    resolved_nodes_for_theme(snapshot, "default")
        .unwrap()
        .into_iter()
        .find(|n| n.id == "text")
        .unwrap()
        .props
}

#[test]
fn exact_faces_style_and_direct_base_precedence() {
    let mut s = fixture();
    s.styles.push(Style {
        id: "body".into(),
        name: "Body".into(),
        kind: "text".into(),
        value: json!({"fontId":"f400","fontSize":14,"lineHeight":20}),
    });
    for (authored, expected) in [
        (json!({"fontId":"f400","fontWeight":600}), "f600"),
        (
            json!({"fontFamily":"Test Sans","fontWeight":700,"fontStyle":"italic"}),
            "f700i",
        ),
        (json!({"fontFamily":"Test Sans"}), "f400"),
        (json!({"styleId":"body","fontWeight":600}), "f600"),
        (
            json!({"styleId":"body","fontFamily":"Test Sans","fontWeight":700,"fontStyle":"italic"}),
            "f700i",
        ),
        (json!({"fontId":"f600","fontWeight":600.0}), "f600"),
    ] {
        props(&mut s, authored);
        assert_eq!(resolved(&s)["fontId"], expected);
    }
    s.styles[0].value = json!({"fontFamily":"Test Sans","fontWeight":700,"fontStyle":"italic"});
    props(&mut s, json!({"styleId":"body"}));
    assert_eq!(resolved(&s)["fontId"], "f700i");
    props(
        &mut s,
        json!({"styleId":"body","fontId":"font_bundled","fontWeight":400,"fontStyle":"normal"}),
    );
    assert_eq!(resolved(&s)["fontId"], "font_bundled");
}

#[test]
fn errors_are_exact_and_duplicate_base_wins_without_changing_legacy_props() {
    let mut s = fixture();
    for (authored, field, fragment) in [
        (
            json!({"fontFamily":"Test Sans","fontWeight":800}),
            "fontWeight",
            "imported faces: 400 normal, 600 normal, 700 italic",
        ),
        (
            json!({"fontId":"f400","fontStyle":"italic"}),
            "fontStyle",
            "Import the face",
        ),
        (
            json!({"fontId":"f400","fontFamily":"Test Sans"}),
            "props.fontFamily",
            "not both",
        ),
        (
            json!({"fontFamily":"Test Sans","fontWeight":950}),
            "fontWeight",
            "integer",
        ),
        (
            json!({"fontFamily":"Test Sans","fontWeight":600.5}),
            "fontWeight",
            "integer",
        ),
        (
            json!({"fontFamily":"Test Sans","fontStyle":"oblique"}),
            "fontStyle",
            "normal or italic",
        ),
        (json!({}), "fontId", "fontFamily"),
    ] {
        props(&mut s, authored);
        let e = resolved_nodes_for_theme(&s, "default").unwrap_err();
        assert_eq!(e.field.as_deref(), Some(field));
        assert!(e.message.contains(fragment), "{e}");
    }
    props(&mut s, json!({"fontFamily":"test sans"}));
    let e = resolved_nodes_for_theme(&s, "default").unwrap_err();
    assert_eq!(e.node.unwrap().suggestion.as_deref(), Some("Test Sans"));
    let mut duplicate = s.fonts.iter().find(|f| f.id == "f600").unwrap().clone();
    duplicate.id = "dup600".into();
    s.fonts.push(duplicate);
    props(&mut s, json!({"fontFamily":"Test Sans","fontWeight":600}));
    let e = resolved_nodes_for_theme(&s, "default").unwrap_err();
    assert!(e.message.contains("f600") && e.message.contains("dup600"));
    props(&mut s, json!({"fontId":"dup600","fontWeight":600}));
    assert_eq!(resolved(&s)["fontId"], "dup600");
    props(&mut s, json!({"fontId":"dup600"}));
    let authored = s
        .nodes
        .iter()
        .find(|n| n.id == "text")
        .unwrap()
        .props
        .clone();
    assert_eq!(resolved(&s), authored);
}

#[test]
fn themes_and_instance_style_overrides_choose_effective_faces() {
    let mut s = fixture();
    s.tokens.push(Token {
        id: "weight".into(),
        name: "Weight".into(),
        token_type: "number".into(),
        value: json!({"default":400,"themes":{"bold":600}}),
    });
    s.styles.push(Style {
        id: "body".into(),
        name: "Body".into(),
        kind: "text".into(),
        value: json!({"fontFamily":"Test Sans","fontWeight":{"tokenId":"weight"}}),
    });
    s.styles.push(Style {
        id: "italic".into(),
        name: "Italic".into(),
        kind: "text".into(),
        value: json!({"fontFamily":"Test Sans","fontWeight":700,"fontStyle":"italic"}),
    });
    s.nodes.iter_mut().find(|n| n.id == "frame").unwrap().props["component"] =
        json!({"name":"Card"});
    props(&mut s, json!({"styleId":"body"}));
    s.nodes.push(Node {
        id: "instance".into(),
        parent_id: Some("page".into()),
        kind: NodeKind::Instance,
        sort_key: 1,
        props: json!({"componentId":"frame","overrides":{"text":{"styleId":"italic"}}}),
    });
    for theme in ["default", "bold"] {
        let nodes = resolved_expanded_nodes(&s, theme).unwrap();
        assert_eq!(
            nodes.iter().find(|n| n.id == "text").unwrap().props["fontId"],
            if theme == "bold" { "f600" } else { "f400" }
        );
        assert_eq!(
            nodes
                .iter()
                .find(|n| n.id == "instance/text")
                .unwrap()
                .props["fontId"],
            "f700i"
        );
    }
    let expanded = instances::expand(&s.nodes).unwrap();
    let refs = referenced_font_ids(&s, &expanded.iter().collect::<Vec<_>>()).unwrap();
    assert_eq!(
        refs.into_iter().collect::<Vec<_>>(),
        vec!["f400", "f600", "f700i"]
    );
    s.tokens[0].value["themes"]["missing"] = json!(800);
    assert!(resolved_expanded_nodes(&s, "default").is_err());
}
