//! P08a `align:"stretch"` sizing and the direct stack guards.
use super::*;
use crate::stack_tests::{input, node, rect, run};
use serde_json::{Value, json};

#[test]
fn stretch_fills_only_unset_cross_sizes() {
    let layout = json!({"mode":"column","padding":10,"gap":4,"align":"stretch"});
    let kids = json!([
        node("unset", "f", "rectangle", json!({"height":10})),
        node("hug", "f", "rectangle", json!({"width":"hug","height":10})),
        rect("fixed", "f", 50., 10.),
        node("max", "f", "rectangle", json!({"maxWidth":100,"height":10})),
        node(
            "fill",
            "f",
            "rectangle",
            json!({"width":"fill","height":10})
        ),
        node(
            "para",
            "f",
            "text",
            json!({"text":"abcdefghijklmnopqrstuvw"})
        ),
        node(
            "label",
            "f",
            "text",
            json!({"width":"hug","text":"Hello world"})
        )
    ]);
    let b = run(input((200., 300.), layout, kids));
    let widths = ["unset", "hug", "fixed", "max", "fill"].map(|id| b[id][2]);
    assert_eq!(widths, [180., 0., 50., 100., 180.]);
    assert_eq!((b["para"][2], b["para"][3]), (180., 48.));
    assert_eq!((b["label"][2], b["label"][3]), (88., 24.));
    let layout = json!({"mode":"row","padding":5,"align":"stretch"});
    let kids = json!([
        node("unset", "f", "rectangle", json!({"width":10})),
        node("hug", "f", "rectangle", json!({"width":10,"height":"hug"})),
        rect("fixed", "f", 10., 30.)
    ]);
    let b = run(input((300., 60.), layout, kids));
    assert_eq!(["unset", "hug", "fixed"].map(|id| b[id][3]), [50., 0., 30.]);
}

#[test]
fn hug_stack_stretches_items_to_the_widest() {
    let item = json!({"height":24,"layout":{"mode":"row","padding":[0,8]}});
    let b = run(input(
        (400., 300.),
        json!({"mode":"column"}),
        json!([
            node(
                "menu",
                "f",
                "group",
                json!({"width":"hug","height":"hug",
                "layout":{"mode":"column","padding":4,"align":"stretch"}})
            ),
            node("i1", "menu", "group", item.clone()),
            node("i2", "menu", "group", item),
            rect("a", "i1", 60., 10.),
            rect("b", "i2", 120., 10.)
        ]),
    ));
    assert_eq!((b["menu"][2], b["menu"][3]), (144., 56.));
    assert_eq!((b["i1"][2], b["i2"][2]), (136., 136.));
}

#[test]
fn wrap_stretch_equalizes_heights_per_line() {
    let card = |id: &str| {
        node(
            id,
            "f",
            "group",
            json!({"width":120,"layout":{"mode":"column"}}),
        )
    };
    let b = run(input(
        (300., 200.),
        json!({"mode":"row","wrap":true,"gap":10,"align":"stretch"}),
        json!([
            card("a"),
            card("b"),
            card("c"),
            rect("ra", "a", 10., 40.),
            rect("rb", "b", 10., 60.),
            rect("rc", "c", 10., 30.)
        ]),
    ));
    assert_eq!(["a", "b", "c"].map(|id| b[id][3]), [60., 60., 30.]);
}

/// Text without a font fails later, so `LAYOUT_INVALID` proves the guard ran first.
fn guarded(layout: Value) -> Value {
    let mut v = input(
        (200., 100.),
        layout,
        json!([node("t", "f", "text", json!({"text":"x"}))]),
    );
    v["fonts"] = json!([]);
    v
}

#[test]
fn invalid_stack_rules_fail_before_measurement() {
    for layout in [
        json!({"mode":"column","padding":[]}),
        json!({"mode":"column","padding":[1,2,3,4,5]}),
        json!({"mode":"column","padding":[1,-2]}),
        json!({"mode":"row","align":"middle"}),
        json!({"mode":"row","justify":"space-around"}),
        json!({"mode":"free","align":"center"}),
        json!({"mode":"free","justify":"start"}),
    ] {
        let value: LayoutInput = serde_json::from_value(guarded(layout.clone())).unwrap();
        let error = compute(&value, &MeasureCache::new()).unwrap_err();
        assert_eq!(error.code, "LAYOUT_INVALID", "{layout}");
    }
    for layout in [
        json!({"mode":"row","padding":"16"}),
        json!({"mode":"row","align":1}),
    ] {
        assert!(serde_json::from_value::<LayoutInput>(guarded(layout)).is_err());
    }
}
