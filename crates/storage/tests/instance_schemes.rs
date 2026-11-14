mod common;
mod schemes;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};

fn props<'a>(nodes: &'a [Node], id: &str) -> &'a Value {
    &nodes.iter().find(|n| n.id == id).unwrap().props
}

#[test]
fn placed_scopes_merge_paint_states_styles_and_keep_authored_source() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "fixture", schemes::fixture()))
        .unwrap();
    let authored = p.snapshot().unwrap();
    for global in ["default", "night", "day"] {
        let nodes = resolved_expanded_nodes(&authored, global).unwrap();
        let root = props(&nodes, "one");
        assert_eq!(root["effectiveTheme"], "night");
        assert_eq!(root["fill"], "#abcdef");
        assert_eq!(
            root["stroke"],
            json!({"color":"#123456","width":2,"align":"outside"})
        );
        assert_eq!(root["radius"], 8);
        assert_eq!(root["opacity"], 0.9);
        assert_eq!(root["shadow"][0]["color"], "#eeeeee");
        let box_props = props(&nodes, "one/box");
        assert_eq!(
            box_props["stroke"],
            json!({"color":"#f00","width":2,"align":"center"})
        );
        assert_eq!(box_props["shadow"], json!([{"blur":6,"color":"#eeeeee"}]));
        assert_eq!(
            box_props["states"]["hover"],
            json!({"fill":"#00f","opacity":0.4,"stroke":{"color":"#0f0","width":3}})
        );
        assert_eq!(box_props["states"]["focus"]["stroke"]["color"], "#eeeeee");
        assert_eq!(box_props["states"]["pressed"]["opacity"], 0.2);
        assert_eq!(box_props["radius"], json!([1, 2]));
        assert_eq!(box_props["opacity"], 0.7);
        assert_eq!(props(&nodes, "one/nested")["fill"], "#eeeeee");
        assert_eq!(props(&nodes, "one/inner")["fill"], "#111111");
        assert_eq!(props(&nodes, "one/label")["fill"], "#eeeeee");
        assert_eq!(props(&nodes, "one/label")["fontId"], "font_bundled");
        assert_eq!(props(&nodes, "two/box")["fill"], "#008800");
        assert_eq!(props(&nodes, "box")["fill"], "#008800");
    }
    assert_eq!(p.snapshot().unwrap().nodes, authored.nodes);
    assert_eq!(
        props(&authored.nodes, "box")["stroke"]["color"],
        json!({"tokenId":"ink"})
    );
    let all: Vec<_> = instances::expand(&authored.nodes).unwrap();
    assert_eq!(
        referenced_font_ids(&authored, &all.iter().collect::<Vec<_>>()).unwrap(),
        std::collections::BTreeSet::from(["font_bundled".into()])
    );
}

#[test]
fn explicit_instance_theme_wins_master_root_and_nearest_nested_scope() {
    let (_temp, _store, p) = setup();
    let mut ops = schemes::fixture();
    ops.push(Operation::UpdateNode {
        merge: false,
        id: "master".into(),
        props: json!({"theme":"default"}),
    });
    p.apply_batch(request(&p, 0, "fixture", ops)).unwrap();
    let nodes = resolved_expanded_nodes(&p.snapshot().unwrap(), "night").unwrap();
    assert_eq!(props(&nodes, "one/box")["effectiveTheme"], "default");
    assert_eq!(props(&nodes, "two/box")["effectiveTheme"], "day");
    assert_eq!(props(&nodes, "two/inner")["effectiveTheme"], "default");
}

#[test]
fn invalid_themes_and_full_merged_paint_fail_atomically_with_override_paths() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "fixture", schemes::fixture()))
        .unwrap();
    let before = p.snapshot().unwrap();
    for (i, (patch, code, field)) in [
        (json!({"theme":"unknown"}), "UNKNOWN_THEME", "props.theme"),
        (json!({"theme":42}), "VALIDATION_FAILED", "props.theme"),
        (
            json!({"overrides":{"box":{"stroke":{"color":"red"}}}}),
            "VALIDATION_FAILED",
            "props.overrides.box.stroke.color",
        ),
        (
            json!({"overrides":{"box":{"states":{"hover":{"shadow":[]}}}}}),
            "VALIDATION_FAILED",
            "props.overrides.box.states.hover.shadow",
        ),
        (
            json!({"overrides":{"label":{"stroke":{"color":"#fff"}}}}),
            "VALIDATION_FAILED",
            "props.overrides.label.stroke",
        ),
        (
            json!({"overrides":{"box":{"radius":-1}}}),
            "VALIDATION_FAILED",
            "props.overrides.box.radius",
        ),
        (
            json!({"overrides":{"box":{"opacity":2}}}),
            "VALIDATION_FAILED",
            "props.overrides.box.opacity",
        ),
        (
            json!({"overrides":{"box":{"layout":{"mode":"row"}}}}),
            "VALIDATION_FAILED",
            "props.overrides.box.layout",
        ),
    ]
    .into_iter()
    .enumerate()
    {
        let error = p
            .apply_batch(request(
                &p,
                1,
                &format!("bad{i}"),
                vec![
                    Operation::UpdateNode {
                        merge: false,
                        id: "label".into(),
                        props: json!({"text":"must rollback"}),
                    },
                    Operation::UpdateNode {
                        merge: false,
                        id: "one".into(),
                        props: patch,
                    },
                ],
            ))
            .unwrap_err();
        assert_eq!(error.code, code, "{error:?}");
        assert_eq!(error.field.as_deref(), Some(field), "{error:?}");
        assert_eq!(p.snapshot().unwrap().nodes, before.nodes);
        assert_eq!(p.snapshot().unwrap().revision, 1);
    }
    assert_eq!(
        resolved_expanded_nodes(&before, "unknown")
            .unwrap_err()
            .code,
        "UNSUPPORTED_THEME"
    );
}
