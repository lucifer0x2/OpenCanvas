//! P08a: per-side padding, align and justify in the authored `layout` object.
mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};

fn update(id: &str, props: Value) -> Operation {
    Operation::UpdateNode {
        id: id.into(),
        props,
    }
}
fn token(id: &str, kind: &str, value: Value) -> Operation {
    Operation::UpsertToken {
        token: Token {
            id: id.into(),
            name: id.into(),
            token_type: kind.into(),
            value,
        },
    }
}
fn fixture() -> Vec<Operation> {
    vec![
        token(
            "space",
            "dimension",
            json!({"default":12,"themes":{"dark":20}}),
        ),
        token("ink", "color", json!("#111111")),
        create("page", NodeKind::Page, None, json!({})),
        create(
            "frame",
            NodeKind::Frame,
            Some("page"),
            json!({"width":400,"height":300,"layout":{"mode":"column"}}),
        ),
        create(
            "menu",
            NodeKind::Group,
            Some("frame"),
            json!({"width":"hug","height":"hug","layout":{"mode":"column"}}),
        ),
    ]
}
fn layout(value: Value) -> Operation {
    update("frame", json!({ "layout": value }))
}
/// Rejected atomically: snapshot, revision and log unchanged.
fn reject(p: &Project, key: &str, op: Operation) -> StorageError {
    let before = p.snapshot().unwrap();
    let commits = serde_json::to_value(p.commits_after(0, 100).unwrap()).unwrap();
    let ops = vec![update("frame", json!({"name":"must roll back"})), op];
    let error = p
        .apply_batch(request(p, before.revision, key, ops))
        .unwrap_err();
    assert_eq!(error.code, "VALIDATION_FAILED", "{key}: {error:?}");
    assert_eq!(p.snapshot().unwrap(), before);
    let after = serde_json::to_value(p.commits_after(0, 100).unwrap()).unwrap();
    assert_eq!(after, commits);
    error
}
fn suggestion(e: &StorageError) -> Option<&str> {
    e.node.as_ref().and_then(|n| n.suggestion.as_deref())
}

#[test]
fn padding_forms_and_alignment_keywords_are_accepted() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", fixture())).unwrap();
    let mut ops = Vec::new();
    for padding in [
        json!(8),
        json!([8, 16]),
        json!([8, 16, 24]),
        json!([8, 16, 24, 32]),
        json!([{"tokenId":"space"}, 16]),
    ] {
        ops.push(layout(json!({"mode":"column","padding":padding})));
    }
    for mode in ["row", "column"] {
        for align in ["start", "center", "end", "stretch"] {
            for justify in ["start", "center", "end", "space-between"] {
                ops.push(layout(json!({"mode":mode,"align":align,"justify":justify})));
            }
        }
    }
    for (i, op) in ops.into_iter().enumerate() {
        let revision = p.snapshot().unwrap().revision;
        p.apply_batch(request(&p, revision, &format!("ok-{i}"), vec![op]))
            .unwrap();
    }
}

#[test]
fn invalid_layout_values_name_the_field_and_a_fix() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", fixture())).unwrap();
    for (i, padding) in [
        json!([]),
        json!([1, 2, 3, 4, 5]),
        json!(-1),
        json!(100001),
        json!("8"),
        json!([{"tokenId":"ink"}, 4]),
    ]
    .into_iter()
    .enumerate()
    {
        let op = layout(json!({"mode":"column","padding":padding}));
        let e = reject(&p, &format!("pad-{i}"), op);
        assert_eq!(e.field.as_deref(), Some("layout.padding"), "{padding}");
        assert!(e.message.contains("[top, right, bottom, left]"));
    }
    let e = reject(
        &p,
        "align",
        layout(json!({"mode":"row","align":"flex-start"})),
    );
    assert_eq!(e.field.as_deref(), Some("layout.align"));
    assert_eq!(
        e.message,
        r#"layout.align must be "start", "center", "end" or "stretch""#
    );
    assert_eq!(suggestion(&e), Some("start"));
    let e = reject(
        &p,
        "justify",
        layout(json!({"mode":"row","justify":"SPACE_BETWEEN"})),
    );
    assert_eq!(e.field.as_deref(), Some("layout.justify"));
    assert_eq!(suggestion(&e), Some("space-between"));
    let e = reject(
        &p,
        "around",
        layout(json!({"mode":"row","justify":"space-around"})),
    );
    assert_eq!(suggestion(&e), None);
    let e = reject(&p, "free", layout(json!({"mode":"free","align":"center"})));
    assert_eq!(e.field.as_deref(), Some("layout.align"));
    assert!(e.message.contains("free children use x/y"));
}

#[test]
fn unknown_keys_missing_mode_and_fill_in_hug_explain_the_fix() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", fixture())).unwrap();
    let e = reject(
        &p,
        "alias",
        layout(json!({"mode":"row","alignItems":"center"})),
    );
    assert_eq!(e.field.as_deref(), Some("layout.alignItems"));
    assert_eq!(suggestion(&e), Some("align"));
    let e = reject(&p, "side", layout(json!({"mode":"row","paddingLeft":8})));
    assert_eq!(e.field.as_deref(), Some("layout.paddingLeft"));
    assert_eq!(suggestion(&e), Some("padding"));
    assert!(e.message.contains("[top, right, bottom, left]"));
    let e = reject(&p, "spacing", layout(json!({"mode":"row","itemSpacing":8})));
    assert_eq!(suggestion(&e), Some("gap"));
    let e = reject(&p, "partial", layout(json!({"align":"center"})));
    assert_eq!(e.field.as_deref(), Some("layout.mode"));
    assert!(e.message.contains("replaces the whole layout object"));
    let child = create(
        "item",
        NodeKind::Group,
        Some("menu"),
        json!({"width":"fill","height":24}),
    );
    let e = reject(&p, "fill-hug", child);
    assert_eq!(e.field.as_deref(), Some("width"));
    assert!(e.message.contains(r#"align to "stretch""#), "{}", e.message);
    let top = reject(
        &p,
        "top-level",
        update("frame", json!({"alignItems":"center"})),
    );
    assert_eq!(suggestion(&top), Some("layout"));
}

#[test]
fn padding_tokens_resolve_per_theme_and_authored_list_is_kept() {
    let (_temp, _store, p) = setup();
    let mut ops = fixture();
    ops.push(layout(
        json!({"mode":"row","padding":[{"tokenId":"space"},16],
        "align":"center","justify":"space-between"}),
    ));
    p.apply_batch(request(&p, 0, "initial", ops)).unwrap();
    let snapshot = p.snapshot().unwrap();
    let authored = &snapshot
        .nodes
        .iter()
        .find(|n| n.id == "frame")
        .unwrap()
        .props;
    assert_eq!(
        authored["layout"]["padding"],
        json!([{"tokenId":"space"}, 16])
    );
    for (theme, want) in [("default", 12), ("dark", 20)] {
        let nodes = resolved_nodes_for_theme(&snapshot, theme).unwrap();
        let frame = nodes.iter().find(|n| n.id == "frame").unwrap();
        assert_eq!(
            frame.props["layout"]["padding"],
            json!([want, 16]),
            "{theme}"
        );
        assert_eq!(frame.props["layout"]["align"], "center");
    }
}
