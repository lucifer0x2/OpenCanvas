mod common;
use common::{create, request, setup};
use opencanvas_storage::*;
use serde_json::{json, Value};

fn update(id: &str, props: Value) -> Operation {
    Operation::UpdateNode {
        merge: false,
        id: id.into(),
        props,
    }
}

fn token(id: &str, value: Value) -> Operation {
    Operation::UpsertToken {
        token: Token {
            id: id.into(),
            name: id.into(),
            token_type: "dimension".into(),
            value,
        },
    }
}

fn reject(p: &Project, bad: Operation) {
    let before = p.snapshot().unwrap();
    let commits = serde_json::to_value(p.commits_after(0, 100).unwrap()).unwrap();
    let error = p
        .apply_batch(request(
            p,
            before.revision,
            "retryable",
            vec![update("frame", json!({"name":"must roll back"})), bad],
        ))
        .unwrap_err();
    assert_eq!(error.code, "VALIDATION_FAILED", "{error:?}");
    assert!(error.field.is_some(), "{error:?}");
    assert_eq!(p.snapshot().unwrap(), before);
    assert_eq!(
        serde_json::to_value(p.commits_after(0, 100).unwrap()).unwrap(),
        commits
    );
}

fn fixture() -> Vec<Operation> {
    vec![
        token("pad", json!({"default":12,"themes":{"dark":18}})),
        token("min", json!({"default":40,"themes":{"dark":48}})),
        create(
            "page",
            NodeKind::Page,
            None,
            json!({"width":1200,"height":800}),
        ),
        create(
            "frame",
            NodeKind::Frame,
            Some("page"),
            json!({
                "width":1200,"height":800,"layout":{"mode":"free"}
            }),
        ),
        create(
            "row",
            NodeKind::Group,
            Some("frame"),
            json!({
                "width":"75.5%","height":"fill","layout":{"mode":"column","breakpoints":[
                    {"maxWidth":600,"layout":{"mode":"row","wrap":true,"padding":[4,{"tokenId":"pad"}],"gap":{"tokenId":"pad"}}}
                ]}
            }),
        ),
        create(
            "card",
            NodeKind::Group,
            Some("row"),
            json!({
                "width":"fill","minWidth":{"tokenId":"min"},"height":"50%"
            }),
        ),
        create(
            "item",
            NodeKind::Rectangle,
            Some("card"),
            json!({"width":"100%","height":"0%"}),
        ),
    ]
}

#[test]
fn themes_resolve_full_alternates_and_final_validation_is_atomic() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", fixture())).unwrap();
    let snapshot = p.snapshot().unwrap();
    let row = snapshot.nodes.iter().find(|n| n.id == "row").unwrap();
    assert_eq!(
        row.props["layout"]["breakpoints"][0]["layout"]["gap"],
        json!({"tokenId":"pad"})
    );
    for (theme, expected) in [("default", 12), ("dark", 18)] {
        let nodes = resolved_nodes_for_theme(&snapshot, theme).unwrap();
        let row = nodes.iter().find(|n| n.id == "row").unwrap();
        assert_eq!(row.props["width"], "75.5%");
        let rule = &row.props["layout"]["breakpoints"][0]["layout"];
        assert_eq!(rule["padding"], json!([4, expected]));
        assert_eq!(rule["gap"], expected);
    }
    reject(&p, token("min", json!({"default":40,"themes":{"dark":0}})));
    reject(&p, update("row", json!({"height":"hug"})));
    reject(&p, update("frame", json!({"width": "75%"})));
    let applied = p
        .apply_batch(request(
            &p,
            1,
            "retryable",
            vec![update("frame", json!({"name":"valid"}))],
        ))
        .unwrap();
    assert_eq!(applied.committed_revision, 2);
    assert!(!applied.replayed);
}

#[test]
fn strict_shapes_and_sizing_reject_invalid_values_in_every_rule() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", fixture())).unwrap();
    let variant = json!({"maxWidth":500,"layout":{"mode":"row"}});
    for invalid in [
        json!(null),
        json!([variant.clone(), variant.clone()]),
        json!([{"maxWidth":600,"layout":{"mode":"free"}},variant.clone()]),
        Value::Array(vec![variant; 9]),
        json!([{"maxWidth":0,"layout":{"mode":"column"}}]),
        json!([{"maxWidth":100001,"layout":{"mode":"column"}}]),
        json!([{"maxWidth":500.5,"layout":{"mode":"column"}}]),
        json!([{"maxWidth":500,"layout":{"mode":"column","breakpoints":[]}}]),
        json!([{"maxWidth":500,"layout":{"padding":4}}]),
        json!([{"maxWidth":500,"layout":{"mode":"column","wrap":true}}]),
        json!([{"maxWidth":500,"layout":{"mode":"row","padding":[{"tokenId":"missing"}]}}]),
        json!([{"maxWidth":500,"layout":{"mode":"row","unknown":true}}]),
        json!([{"maxWidth":500,"layout":{"mode":"row"},"extra":1}]),
    ] {
        reject(
            &p,
            update(
                "frame",
                json!({"layout":{"mode":"free","breakpoints":invalid}}),
            ),
        );
    }
    for width in ["-1%", "100.001%", "0x10%", "1e2%", "50% ", "calc(50%)"] {
        reject(&p, update("item", json!({"width":width})));
    }
}

#[test]
fn component_percentage_axis_is_checked_at_actual_placement() {
    let (_temp, _store, p) = setup();
    let mut ops = fixture();
    ops.extend([
        create(
            "master",
            NodeKind::Frame,
            Some("page"),
            json!({
                "component":{"name":"Responsive panel"},"width":400,"height":80,
                "layout":{"mode":"free","breakpoints":[{"maxWidth":600,"layout":{"mode":"column"}}]}
            }),
        ),
        create(
            "content",
            NodeKind::Rectangle,
            Some("master"),
            json!({"width":"50%","height":20}),
        ),
        create(
            "panel",
            NodeKind::Instance,
            Some("frame"),
            json!({"componentId":"master","width":"50%"}),
        ),
    ]);
    p.apply_batch(request(&p, 0, "placed", ops)).unwrap();
    let nodes = resolved_expanded_nodes(&p.snapshot().unwrap(), "default").unwrap();
    let child = nodes
        .iter()
        .find(|node| node.id == "panel/content")
        .unwrap();
    assert_eq!(child.props["width"], "50%");
    // The same definition is valid, but a hug width at the placement creates a cycle.
    reject(&p, update("panel", json!({"width":"hug"})));
}
