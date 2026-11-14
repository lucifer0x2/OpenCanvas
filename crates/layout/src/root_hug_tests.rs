//! A scene root with `height:"hug"` takes its content height; the viewport keeps the width.
use super::*;
use crate::stack_tests::{input, node, rect, run, scene};
use serde_json::{Value, json};

/// `input` with a hug root height and extra root props.
fn hug(viewport: (f32, f32), layout: Value, nodes: Value, root: Value) -> Value {
    let mut value = input(viewport, layout, nodes);
    let props = value["nodes"][0]["props"].as_object_mut().unwrap();
    props.insert("height".into(), json!("hug"));
    props.extend(root.as_object().unwrap().clone());
    value
}

fn surface(value: Value) -> (f64, f64, [f32; 4]) {
    let scene = scene(value);
    let root = &scene.nodes[0].bounds;
    let viewport = &scene.provenance.viewport;
    (
        viewport.width,
        viewport.height,
        [root.x, root.y, root.width, root.height],
    )
}

fn error(value: Value) -> String {
    let input: LayoutInput = serde_json::from_value(value).unwrap();
    compute(&input, &MeasureCache::new()).unwrap_err().message
}

#[test]
fn column_root_height_is_padding_gaps_and_children_whatever_the_viewport_height() {
    let children = json!([rect("a", "f", 100., 40.), rect("b", "f", 50., 20.)]);
    let layout = json!({"mode":"column","padding":16,"gap":8});
    for height in [1., 844., 5000.] {
        let value = hug((390., height), layout.clone(), children.clone(), json!({}));
        assert_eq!(surface(value), (390., 100., [0., 0., 390., 100.]));
    }
    let row = hug(
        (390., 844.),
        json!({"mode":"row","padding":[4, 0]}),
        children,
        json!({}),
    );
    assert_eq!(surface(row).1, 48.);
}

#[test]
fn nested_hug_text_wraps_at_the_viewport_width() {
    let nodes = json!([
        node(
            "card",
            "f",
            "group",
            json!({"width":"fill","height":"hug","layout":{"mode":"column","padding":5}})
        ),
        node(
            "t",
            "card",
            "text",
            json!({"width":"fill","height":"hug","text":"x".repeat(40)})
        ),
    ]);
    let value = hug(
        (100., 600.),
        json!({"mode":"column","padding":10}),
        nodes,
        json!({}),
    );
    let boxes = run(value.clone());
    // 70 px of text width holds 8 characters per 24 px line: five lines.
    assert_eq!(boxes["t"], [15., 15., 70., 120.]);
    assert_eq!(boxes["card"], [10., 10., 80., 130.]);
    assert_eq!(surface(value).1, 150.);
}

#[test]
fn empty_root_is_one_pixel_and_min_max_height_clamp() {
    let empty = hug((390., 844.), json!({"mode":"column"}), json!([]), json!({}));
    assert_eq!(surface(empty), (390., 1., [0., 0., 390., 1.]));
    let tall = json!([rect("a", "f", 10., 100.)]);
    let clamp = |root| {
        surface(hug(
            (390., 844.),
            json!({"mode":"column"}),
            tall.clone(),
            root,
        ))
    };
    assert_eq!(clamp(json!({"maxHeight":50})).1, 50.);
    assert_eq!(clamp(json!({"minHeight":300})).1, 300.);
}

#[test]
fn breakpoints_reflow_and_the_height_follows() {
    let layout = json!({"mode":"row","breakpoints":[{"maxWidth":400,"layout":{"mode":"column"}}]});
    let nodes = json!([rect("a", "f", 100., 40.), rect("b", "f", 100., 60.)]);
    let wide = hug((800., 600.), layout.clone(), nodes.clone(), json!({}));
    let narrow = hug((375., 600.), layout, nodes, json!({}));
    assert_eq!(surface(wide).1, 60.);
    assert_eq!(surface(narrow).1, 100.);
}

#[test]
fn hug_root_height_is_not_a_definite_parent_axis() {
    let layout = json!({"mode":"column"});
    for (props, message) in [
        (
            json!({"width":10,"height":"50%"}),
            "Percentage requires a definite parent axis",
        ),
        (
            json!({"width":10,"height":"fill"}),
            "Fill inside hug axis is ambiguous",
        ),
    ] {
        let child = json!([node("c", "f", "rectangle", props)]);
        assert_eq!(
            error(hug((390., 844.), layout.clone(), child, json!({}))),
            message
        );
    }
    // The width still comes from the viewport.
    let child = json!([node(
        "c",
        "f",
        "rectangle",
        json!({"width":"50%","height":10})
    )]);
    assert_eq!(
        run(hug((390., 844.), layout, child, json!({})))["c"][2],
        195.
    );
}

#[test]
fn computed_height_stays_within_the_pixel_budget() {
    let tall = json!([rect("a", "f", 10., 2000.)]);
    let value = hug((10_000., 1.), json!({"mode":"column"}), tall, json!({}));
    assert_eq!(error(value), "Hug root height exceeds the viewport budget");
}
