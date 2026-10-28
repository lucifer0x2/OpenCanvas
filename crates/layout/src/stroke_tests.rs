use super::*;
use serde_json::{Value, json};

fn scene(frame_props: Value) -> Scene {
    let input: LayoutInput = serde_json::from_value(json!({
        "schemaVersion": 1, "projectId": "p", "revision": 1, "frameId": "root",
        "viewport": {"width": 300, "height": 200},
        "nodes": [
            {"id": "root", "parentId": null, "kind": "frame", "props": {}},
            {"id": "box", "parentId": "root", "kind": "frame", "props": frame_props},
            {"id": "child", "parentId": "box", "kind": "rectangle",
             "props": {"x": 5, "y": 6, "width": 20, "height": 20}},
        ],
    }))
    .unwrap();
    match compute(&input, &MeasureCache::new()).unwrap() {
        LayoutStep::Ready { scene } => scene,
        LayoutStep::Pending { .. } => panic!("no text, must be ready"),
    }
}

fn bounds(scene: &Scene, id: &str) -> Bounds {
    scene
        .nodes
        .iter()
        .find(|n| n.id == id)
        .unwrap()
        .bounds
        .clone()
}

fn column(stroke: Value, size: Value) -> Value {
    json!({"x": 10, "y": 10, "width": size, "height": size,
           "layout": {"mode": "column", "padding": 8}, "stroke": stroke})
}

#[test]
fn inside_stroke_takes_stack_space_like_a_css_border() {
    let stroke = json!({"color": "#000", "width": [1, 2, 3, 4]});
    let s = scene(column(stroke.clone(), json!(100)));
    let (parent, child) = (bounds(&s, "box"), bounds(&s, "child"));
    assert_eq!(child.x, parent.x + 4.0 + 8.0);
    assert_eq!(child.y, parent.y + 1.0 + 8.0);
    let hug = bounds(&scene(column(stroke, json!("hug"))), "box");
    assert_eq!(
        (hug.width, hug.height),
        (20.0 + 16.0 + 2.0 + 4.0, 20.0 + 16.0 + 1.0 + 3.0)
    );
}

#[test]
fn outside_stroke_is_paint_only() {
    let stroke = json!({"color": "#000", "width": [1, 2, 3, 4], "align": "outside"});
    let s = scene(column(stroke.clone(), json!(100)));
    let (parent, child) = (bounds(&s, "box"), bounds(&s, "child"));
    assert_eq!((child.x, child.y), (parent.x + 8.0, parent.y + 8.0));
    let hug = bounds(&scene(column(stroke, json!("hug"))), "box");
    assert_eq!((hug.width, hug.height), (36.0, 36.0));
}

#[test]
fn free_children_keep_authored_position() {
    let props = json!({"x": 10, "y": 10, "width": 100, "height": 100,
                       "layout": {"mode": "free"}, "stroke": {"color": "#000", "width": 3}});
    let s = scene(props);
    let (parent, child) = (bounds(&s, "box"), bounds(&s, "child"));
    assert_eq!((child.x, child.y), (parent.x + 5.0, parent.y + 6.0));
}

#[test]
fn scene_stroke_is_normalized_and_absent_without_width() {
    let s = scene(json!({"width": 50, "height": 50, "stroke": {"color": "#D0D5DD"}}));
    let node = s.nodes.iter().find(|n| n.id == "box").unwrap();
    let stroke = node.stroke.as_ref().unwrap();
    assert_eq!(stroke.color, ["#D0D5DD"; 4].map(String::from));
    assert_eq!((stroke.width, stroke.align.as_str()), ([1.0; 4], "inside"));
    let wire = serde_json::to_value(&s).unwrap();
    assert!(
        wire["nodes"][0].get("stroke").is_none(),
        "no key without stroke"
    );
    let none = scene(json!({"width": 50, "height": 50,
                            "stroke": {"color": "#000", "width": [0, 0, 0, 0]}}));
    assert!(none.nodes.iter().all(|n| n.stroke.is_none()));
}

#[test]
fn rejects_non_finite_width_and_unknown_align() {
    for stroke in [
        json!({"color": "#000", "width": -1}),
        json!({"color": "#000", "align": "middle"}),
    ] {
        let input: LayoutInput = serde_json::from_value(json!({
            "schemaVersion": 1, "projectId": "p", "revision": 1, "frameId": "f",
            "viewport": {"width": 10, "height": 10},
            "nodes": [{"id": "f", "parentId": null, "kind": "frame", "props": {"stroke": stroke}}],
        }))
        .unwrap();
        assert_eq!(
            compute(&input, &MeasureCache::new()).unwrap_err().code,
            "LAYOUT_INVALID"
        );
    }
}

/// P08a conditional AC: Taffy padding per side = padding + inside stroke; hug adds both.
#[test]
fn inside_stroke_adds_to_per_side_padding() {
    let props = |size: Value, align: &str| {
        json!({"x": 10, "y": 10, "width": size, "height": size,
            "layout": {"mode": "column", "padding": [8, 16, 24, 32],
                       "align": align, "justify": align},
            "stroke": {"color": "#000", "width": [1, 2, 3, 4]}})
    };
    let s = scene(props(json!("hug"), "start"));
    let (parent, child) = (bounds(&s, "box"), bounds(&s, "child"));
    assert_eq!((child.x - parent.x, child.y - parent.y), (36.0, 9.0));
    assert_eq!(
        (parent.width, parent.height),
        (74.0, 56.0) // 20 + 16 + 32 + 2 + 4, 20 + 8 + 24 + 1 + 3
    );
    // Centered within the box less padding and stroke: x 36 + (100-36-18-20)/2, y 9 + (100-9-27-20)/2.
    let s = scene(props(json!(100), "center"));
    let (parent, child) = (bounds(&s, "box"), bounds(&s, "child"));
    assert_eq!((child.x - parent.x, child.y - parent.y), (49.0, 31.0));
}
