mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};
use std::sync::Arc;

fn token(id: &str, token_type: &str, value: Value) -> Operation {
    Operation::UpsertToken {
        token: Token {
            id: id.into(),
            name: id.into(),
            token_type: token_type.into(),
            value,
        },
    }
}

/// A project with the initial tree, a color token, a dimension token and a fill style.
fn project() -> (tempfile::TempDir, Store, Arc<Project>) {
    let (temp, store, p) = setup();
    let mut ops = vec![
        token("line", "color", json!("#D0D5DD")),
        token("hairline", "dimension", json!(2)),
        Operation::UpsertStyle {
            style: Style {
                id: "edge".into(),
                name: "Edge".into(),
                kind: "fill".into(),
                value: json!({"tokenId":"line"}),
            },
        },
    ];
    ops.extend(initial());
    p.apply_batch(request(&p, 0, "initial", ops)).unwrap();
    (temp, store, p)
}

fn rect(id: &str, stroke: Value) -> Operation {
    create(
        id,
        NodeKind::Rectangle,
        Some("frame"),
        json!({"width":40,"height":20,"stroke":stroke}),
    )
}

fn reject(p: &Project, op: Operation) -> StorageError {
    p.apply_batch(request(p, 1, "reject", vec![op]))
        .unwrap_err()
}

#[test]
fn accepts_uniform_per_side_token_and_style_strokes_on_box_kinds() {
    let (_temp, _store, p) = project();
    let sides = json!(["#D0D5DD", "#D0D5DD", "#667085", "#D0D5DD"]);
    let ops = vec![
        rect("a", json!({"color":"#D0D5DD"})),
        rect("b", json!({"color":sides,"width":[1,1,2,1]})),
        rect(
            "c",
            json!({"color":{"tokenId":"line"},"width":{"tokenId":"hairline"},"align":"outside"}),
        ),
        rect("d", json!({"color":{"styleId":"edge"}})),
        rect(
            "e",
            json!({"color":"#000","width":[0,0,{"tokenId":"hairline"},0]}),
        ),
        create(
            "f",
            NodeKind::Frame,
            Some("frame"),
            json!({"width":10,"height":10,"stroke":{"color":"#000"}}),
        ),
        create(
            "g",
            NodeKind::Group,
            Some("frame"),
            json!({"stroke":{"color":"#000","align":"center"}}),
        ),
    ];
    let result = p.apply_batch(request(&p, 1, "strokes", ops)).unwrap();
    assert_eq!(result.committed_revision, 2);
    let s = p.snapshot().unwrap();
    let node = |id: &str| s.nodes.iter().find(|n| n.id == id).unwrap().clone();
    assert_eq!(
        node("c").props["stroke"]["color"],
        json!({"tokenId":"line"}),
        "authored kept"
    );
    let resolved = resolved_nodes(&s).unwrap();
    let stroke = |id: &str| resolved.iter().find(|n| n.id == id).unwrap().props["stroke"].clone();
    assert_eq!(
        stroke("c"),
        json!({"color":"#D0D5DD","width":2,"align":"outside"})
    );
    assert_eq!(stroke("d"), json!({"color":"#D0D5DD"}));
    assert_eq!(stroke("e"), json!({"color":"#000","width":[0,0,2,0]}));
}

#[test]
fn stroke_on_image_is_accepted() {
    let (_temp, _store, p) = project();
    let asset = p.import_asset(asset_request(&p, 1, "png")).unwrap();
    let op = create(
        "pic",
        NodeKind::Image,
        Some("frame"),
        json!({"width":10,"height":10,"assetId":asset.asset.id,"stroke":{"color":"#000"}}),
    );
    p.apply_batch(request(&p, 2, "pic", vec![op])).unwrap();
}

#[test]
fn stroke_on_text_names_the_key_and_says_to_wrap() {
    let (_temp, _store, p) = project();
    let op = Operation::UpdateNode {
        id: "text".into(),
        props: json!({"stroke":{"color":"#000"}}),
    };
    let e = reject(&p, op);
    assert_eq!(e.field.as_deref(), Some("props.stroke"));
    let node = e.node.as_deref().unwrap();
    assert_eq!(node.node_kind.as_deref(), Some("text"));
    assert!(node
        .allowed_keys
        .as_ref()
        .is_some_and(|k| !k.iter().any(|k| k == "stroke")));
    assert!(e.message.contains("wrap text in a frame"), "{}", e.message);
}

#[test]
fn malformed_values_name_the_stroke_field() {
    let (_temp, _store, p) = project();
    for (stroke, field, hint) in [
        (
            json!({"color":"#fff","width":[1,2]}),
            "stroke.width",
            "[top, right, bottom, left]",
        ),
        (json!({"color":"#fff","width":-1}), "stroke.width", ""),
        (
            json!({"color":"#fff","align":"middle"}),
            "stroke.align",
            "inside, center or outside",
        ),
        (
            json!({"color":"#fff","weight":2}),
            "stroke.weight",
            "color, width and align",
        ),
        (json!({"width":1}), "stroke.color", "requires color"),
        (json!({"color":"red"}), "stroke.color", "#RRGGBB"),
        (
            json!({"color":["#fff","#fff","#fff"]}),
            "stroke.color",
            "[top, right, bottom, left]",
        ),
        (json!("1px solid #ddd"), "stroke", "object"),
    ] {
        let e = reject(&p, rect("bad", stroke.clone()));
        assert_eq!(e.field.as_deref(), Some(field), "{stroke}: {e:?}");
        assert!(e.message.contains(hint), "{stroke}: {}", e.message);
        assert_eq!(e.node.as_ref().unwrap().node_id.as_deref(), Some("bad"));
    }
}

#[test]
fn border_aliases_suggest_stroke() {
    let (_temp, _store, p) = project();
    for (key, value) in [
        ("border", json!({"color":"#000"})),
        ("strokeWidth", json!(1)),
    ] {
        let op = create(
            "bad",
            NodeKind::Rectangle,
            Some("frame"),
            json!({key: value}),
        );
        let e = reject(&p, op);
        assert_eq!(e.field.as_deref(), Some(format!("props.{key}").as_str()));
        assert_eq!(
            e.node.as_ref().unwrap().suggestion.as_deref(),
            Some("stroke")
        );
    }
}

#[test]
fn catalog_lists_stroke_for_box_kinds_only() {
    for kind in &NODE_KINDS {
        let listed = allowed_props(kind).contains(&"stroke");
        let expected = !matches!(kind, NodeKind::Page | NodeKind::Text);
        assert_eq!(listed, expected, "{kind:?}");
    }
}
