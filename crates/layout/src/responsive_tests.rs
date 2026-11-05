use super::*;
use crate::stack_tests::{input, node, run};
use serde_json::{Value, json};

fn fixture() -> Value {
    input(
        (600., 400.),
        json!({"mode":"free","padding":24,"breakpoints":[
            {"maxWidth":400,"layout":{"mode":"row","padding":8,"gap":6}},
            {"maxWidth":600,"layout":{"mode":"column","padding":16,"gap":12}}
        ]}),
        json!([
            node(
                "nested",
                "f",
                "frame",
                json!({
                    "x":40,"y":30,"width":"50%","minWidth":300,"height":"50%",
                    "layout":{"mode":"free","padding":12,"breakpoints":[
                        {"maxWidth":600,"layout":{"mode":"row","padding":10}}
                    ]}
                })
            ),
            node(
                "detail",
                "nested",
                "rectangle",
                json!({
                    "x":12,"y":14,"width":"37.5%","height":"50%","maxHeight":90
                })
            ),
            node(
                "aside",
                "f",
                "rectangle",
                json!({"x":360,"y":40,"width":40,"height":20})
            )
        ]),
    )
}

#[test]
fn whole_rules_use_inclusive_viewport_width_and_percentage_parent_boxes() {
    let mut value = fixture();
    let boxes = run(value.clone());
    assert_eq!(boxes["nested"], [16., 16., 300., 184.]);
    assert_eq!(boxes["detail"], [26., 26., 105., 82.]);
    assert_eq!(boxes["aside"], [16., 212., 40., 20.]);

    // Precision must be preserved until choosing the rule, even when Taffy rounds the input.
    value["viewport"]["width"] = json!(600.000_000_01);
    let boxes = run(value.clone());
    assert_eq!(boxes["nested"], [40., 30., 300., 200.]);
    assert_eq!(boxes["detail"], [52., 44., 112.5, 90.]);
    assert_eq!(boxes["aside"], [360., 40., 40., 20.]);

    value["viewport"]["width"] = json!(400);
    assert_eq!(run(value)["aside"], [314., 8., 40., 20.]);
}

#[test]
fn a_nested_frame_becomes_a_viewport_root_when_selected() {
    let mut value = fixture();
    value["frameId"] = json!("nested");
    value["nodes"] = json!([value["nodes"][1].clone(), value["nodes"][2].clone()]);
    value["nodes"][0]["parentId"] = Value::Null;
    let boxes = run(value);
    assert_eq!(boxes["nested"], [0., 0., 600., 400.]);
    assert_eq!(boxes["detail"], [10., 10., 217.5, 90.]);
}

#[test]
fn free_descendants_use_their_in_flow_parents_containing_box() {
    let value = input(
        (600., 400.),
        json!({"mode":"row","padding":20,"gap":12}),
        json!([
            node("aside", "f", "rectangle", json!({"width":100,"height":40})),
            node(
                "panel",
                "f",
                "group",
                json!({"width":"fill","height":80,"layout":{"mode":"free","padding":10}})
            ),
            node(
                "detail",
                "panel",
                "rectangle",
                json!({"x":10,"y":6,"width":"50%","height":"25%"})
            )
        ]),
    );
    let boxes = run(value);
    assert_eq!(boxes["panel"], [132., 20., 448., 80.]);
    assert_eq!(boxes["detail"], [142., 26., 224., 20.]);
}

#[test]
fn invalid_inactive_rules_and_ambiguous_percentage_axes_fail() {
    let base = fixture();
    for layout in [
        json!({"mode":"column","wrap":true}),
        json!({"mode":"row","gap":-1}),
        json!({"mode":"free","justify":"center"}),
    ] {
        let mut value = base.clone();
        value["viewport"]["width"] = json!(900);
        value["nodes"][0]["props"]["layout"]["breakpoints"][0]["layout"] = layout;
        let typed = serde_json::from_value(value).unwrap();
        assert_eq!(
            compute(&typed, &MeasureCache::new()).unwrap_err().code,
            "LAYOUT_INVALID"
        );
    }
    for layout in [
        json!({"gap":10}),
        json!({"mode":"row","typo":true}),
        json!({"mode":"column","breakpoints":[]}),
    ] {
        let mut value = base.clone();
        value["nodes"][0]["props"]["layout"]["breakpoints"][0]["layout"] = layout;
        assert!(serde_json::from_value::<LayoutInput>(value).is_err());
    }
    let mut value = base;
    value["nodes"][1]["props"]["height"] = json!("hug");
    let typed = serde_json::from_value(value).unwrap();
    let error = compute(&typed, &MeasureCache::new()).unwrap_err();
    assert!(error.message.contains("definite parent axis"));
}

#[test]
fn direct_input_keeps_the_legacy_free_default_and_bounds_percentage_syntax() {
    let value = input(
        (600., 400.),
        json!({}),
        json!([node(
            "child",
            "f",
            "rectangle",
            json!({"x":19,"width":"0%","height":"100.0%"})
        )]),
    );
    assert_eq!(run(value)["child"], [19., 0., 0., 400.]);
    for invalid in [
        "-0%",
        " 50%",
        "1e1%",
        ".5%",
        "1.%",
        "100.000000000001%",
        "101%",
    ] {
        assert_eq!(percentage(invalid), None, "{invalid}");
    }
}
