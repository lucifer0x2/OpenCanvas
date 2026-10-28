//! P08a stack padding forms, justify and align (rect fixtures, fake text measurer).
use super::*;
use serde_json::{Value, json};
use std::collections::HashMap;

pub(crate) type Boxes = HashMap<String, [f32; 4]>;

/// A frame `f` sized by the viewport with `layout`, plus `nodes` (ids, parents, props).
pub(crate) fn input(viewport: (f32, f32), layout: Value, nodes: Value) -> Value {
    let mut all = vec![json!({"id":"f","kind":"frame","props":{"layout":layout}})];
    all.extend(nodes.as_array().unwrap().iter().cloned());
    json!({"schemaVersion":1,"projectId":"p","revision":1,"frameId":"f",
        "viewport":{"width":viewport.0,"height":viewport.1},"nodes":all,
        "fonts":[{"id":"font_bundled","family":"Noto Sans","hash":"h","url":"/font.ttf"}]})
}
pub(crate) fn node(id: &str, parent: &str, kind: &str, props: Value) -> Value {
    json!({"id":id,"parentId":parent,"kind":kind,"props":props})
}
pub(crate) fn rect(id: &str, parent: &str, w: f32, h: f32) -> Value {
    node(id, parent, "rectangle", json!({"width":w,"height":h}))
}

/// Runs `compute` to a scene, measuring text at 8 px per character and 24 px per line.
pub(crate) fn run(value: Value) -> Boxes {
    let input: LayoutInput = serde_json::from_value(value).unwrap();
    let mut cache = MeasureCache::new();
    for _ in 0..MAX_PASSES {
        match compute(&input, &cache).unwrap() {
            LayoutStep::Ready { scene } => {
                return scene
                    .nodes
                    .into_iter()
                    .map(|n| {
                        (
                            n.id,
                            [n.bounds.x, n.bounds.y, n.bounds.width, n.bounds.height],
                        )
                    })
                    .collect();
            }
            LayoutStep::Pending { requests } => {
                for r in requests {
                    let chars = r.text.chars().count();
                    let per_line = r
                        .width
                        .map_or(chars.max(1), |w| ((w / 8.0) as usize).max(1));
                    let lines = chars.div_ceil(per_line).max(1);
                    let m = Measurement {
                        width: (chars.min(per_line) * 8) as f32,
                        height: lines as f32 * r.line_height,
                        lines: Vec::new(),
                    };
                    cache.insert(r.key, m);
                }
            }
        }
    }
    panic!("layout did not converge")
}
pub(crate) fn at(b: &Boxes, id: &str) -> (f32, f32) {
    (b[id][0], b[id][1])
}

#[test]
fn padding_forms_follow_css_shorthand() {
    let sides = json!([10, 20, 30, 40]);
    let b = run(input(
        (400., 300.),
        json!({"mode":"column","padding":sides}),
        json!([
            rect("r", "f", 50., 20.),
            node(
                "g",
                "f",
                "group",
                json!({"width":"hug","height":"hug",
                "layout":{"mode":"row","padding":sides}})
            ),
            rect("c", "g", 50., 20.)
        ]),
    ));
    assert_eq!(at(&b, "r"), (40., 10.));
    assert_eq!(b["g"], [40., 30., 110., 60.]);
    assert_eq!(at(&b, "c"), (80., 40.));
    let b = run(input(
        (400., 300.),
        json!({"mode":"column","padding":[8,16]}),
        json!([rect("r", "f", 50., 20.)]),
    ));
    assert_eq!(at(&b, "r"), (16., 8.));
    let b = run(input(
        (400., 300.),
        json!({"mode":"column","padding":[8,16,24]}),
        json!([
            node(
                "g",
                "f",
                "group",
                json!({"width":"hug","height":"hug",
                "layout":{"mode":"column","padding":[8,16,24]}})
            ),
            rect("c", "g", 50., 20.)
        ]),
    ));
    assert_eq!(b["g"], [16., 8., 82., 52.]);
    assert_eq!(at(&b, "c"), (32., 16.));
    for padding in [json!(16), json!([16])] {
        let b = run(input(
            (400., 300.),
            json!({"mode":"column","padding":padding}),
            json!([rect("r", "f", 50., 20.)]),
        ));
        assert_eq!(at(&b, "r"), (16., 16.));
    }
}

#[test]
fn defaults_keep_existing_bounds_and_serialization() {
    let rule: LayoutRule =
        serde_json::from_value(json!({"mode":"column","padding":16,"gap":8})).unwrap();
    let out = serde_json::to_value(rule).unwrap();
    assert_eq!(out, json!({"mode":"column","padding":16.0,"gap":8.0}));
    let b = run(input(
        (400., 300.),
        json!({"mode":"column","padding":20,"gap":10}),
        json!([
            rect("a", "f", 100., 50.),
            node("b", "f", "rectangle", json!({"height":50})),
            node("t", "f", "text", json!({"text":"Hi"}))
        ]),
    ));
    assert_eq!(b["a"], [20., 20., 100., 50.]);
    assert_eq!(b["b"], [20., 80., 0., 50.]);
    assert_eq!(at(&b, "t"), (20., 140.));
}

#[test]
fn justify_and_align_place_children_in_the_content_box() {
    let centered = json!({"mode":"column","padding":20,"align":"center","justify":"center"});
    for (vp, want) in [((400., 300.), (150., 125.)), ((800., 600.), (350., 275.))] {
        let b = run(input(
            vp,
            centered.clone(),
            json!([rect("r", "f", 100., 50.)]),
        ));
        assert_eq!(at(&b, "r"), want);
    }
    for gap in [0, 8] {
        let layout = json!({"mode":"row","padding":[0,16],"gap":gap,
            "align":"center","justify":"space-between"});
        let kids = json!([
            rect("a", "f", 24., 24.),
            rect("b", "f", 100., 20.),
            rect("c", "f", 24., 24.)
        ]);
        let b = run(input((400., 48.), layout, kids));
        assert_eq!(
            [at(&b, "a"), at(&b, "b"), at(&b, "c")],
            [(16., 12.), (150., 14.), (360., 12.)]
        );
    }
    let layout = json!({"mode":"row","padding":[0,16],"gap":8,"align":"end","justify":"end"});
    let kids = json!([rect("a", "f", 24., 24.), rect("b", "f", 24., 20.)]);
    let b = run(input((400., 48.), layout, kids));
    assert_eq!([at(&b, "a"), at(&b, "b")], [(328., 24.), (360., 28.)]);
    let layout = json!({"mode":"column","padding":[10,0,20,0],"gap":5,
        "align":"end","justify":"space-between"});
    let kids = json!([
        rect("a", "f", 50., 40.),
        rect("b", "f", 60., 40.),
        rect("c", "f", 70., 40.)
    ]);
    let b = run(input((200., 300.), layout, kids));
    assert_eq!(
        [at(&b, "a"), at(&b, "b"), at(&b, "c")],
        [(150., 10.), (140., 125.), (130., 240.)]
    );
    let layout = json!({"mode":"row","padding":16,"justify":"space-between"});
    let b = run(input(
        (400., 48.),
        layout,
        json!([rect("a", "f", 10., 10.)]),
    ));
    assert_eq!(at(&b, "a"), (16., 16.));
}

#[test]
fn hug_and_fill_absorb_free_space_and_wrap_aligns_per_line() {
    let b = run(input(
        (400., 300.),
        json!({"mode":"column"}),
        json!([
            node(
                "g",
                "f",
                "group",
                json!({"width":"hug","height":"hug","layout":
                {"mode":"row","padding":[4,8],"gap":6,"justify":"center","align":"end"}})
            ),
            rect("a", "g", 10., 10.),
            rect("b", "g", 20., 20.)
        ]),
    ));
    assert_eq!(b["g"], [0., 0., 52., 28.]);
    assert_eq!([at(&b, "a"), at(&b, "b")], [(8., 14.), (24., 4.)]);
    let layout = json!({"mode":"row","gap":10,"justify":"center"});
    let kids = json!([
        rect("a", "f", 50., 20.),
        node("b", "f", "rectangle", json!({"width":"fill","height":20}))
    ]);
    let b = run(input((400., 100.), layout, kids));
    assert_eq!(
        (b["a"][0], b["a"][2], b["b"][0], b["b"][2]),
        (0., 50., 60., 340.)
    );
    let layout = json!({"mode":"row","wrap":true,"gap":10,"align":"center","justify":"center"});
    let kids = json!([
        rect("a", "f", 120., 40.),
        rect("b", "f", 120., 60.),
        rect("c", "f", 120., 40.)
    ]);
    let b = run(input((300., 200.), layout, kids));
    assert_eq!(
        [at(&b, "a"), at(&b, "b"), at(&b, "c")],
        [(25., 10.), (155., 0.), (90., 70.)]
    );
}

#[test]
fn overflow_is_unsafe_like_css() {
    let layout = json!({"mode":"row","align":"center","justify":"center"});
    let b = run(input(
        (100., 50.),
        layout,
        json!([rect("a", "f", 140., 80.)]),
    ));
    assert_eq!(at(&b, "a"), (-20., -15.));
    let layout = json!({"mode":"row","justify":"end"});
    let b = run(input(
        (100., 50.),
        layout,
        json!([rect("a", "f", 140., 20.)]),
    ));
    assert_eq!(b["a"][0], -40.);
    let layout = json!({"mode":"row","justify":"space-between"});
    let kids = json!([rect("a", "f", 80., 20.), rect("b", "f", 80., 20.)]);
    let b = run(input((100., 50.), layout, kids));
    assert_eq!((b["a"][0], b["b"][0]), (0., 80.));
}
