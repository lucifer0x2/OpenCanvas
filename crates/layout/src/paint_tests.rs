use super::*;
use serde_json::{Value, json};
fn input(props: Value) -> LayoutInput {
    serde_json::from_value(json!({"schemaVersion":1,"projectId":"p","revision":1,"frameId":"f","viewport":{"width":200,"height":120},"nodes":[{"id":"f","parentId":null,"kind":"frame","props":props}]})).unwrap()
}
#[test]
fn paint_defaults_and_shorthand_only_normalize_scene() {
    let i = input(
        json!({"radius":[4,8,12],"stroke":{"color":["#000","#fff"],"width":[1,2]},"fill":{"type":"radial","stops":[[0,"#fff"],[1,"#0000"]]},"shadow":[{"y":4,"color":"#0004"}]}),
    );
    let LayoutStep::Ready { scene } = compute(&i, &MeasureCache::new()).unwrap() else {
        panic!()
    };
    let wire = serde_json::to_value(scene).unwrap();
    let n = &wire["nodes"][0];
    assert_eq!(n["radius"], json!([4., 8., 12., 8.]));
    assert_eq!(n["stroke"]["width"], json!([1., 2., 1., 2.]));
    assert_eq!(
        n["stroke"]["color"],
        json!(["#000", "#fff", "#000", "#fff"])
    );
    assert_eq!(n["fill"]["center"], json!([0.5, 0.5]));
    assert!(n["fill"]["radius"].is_null());
    assert_eq!(
        n["shadow"],
        json!([{"x":0.,"y":4.,"blur":0.,"spread":0.,"color":"#0004","inset":false}])
    );
    assert_eq!(
        serde_json::to_value(i).unwrap()["nodes"][0]["props"]["radius"],
        json!([4., 8., 12.])
    );
}
#[test]
fn rejects_direct_invalid_paint_without_storage() {
    for props in [
        json!({"radius":[]}),
        json!({"radius":[-1]}),
        json!({"shadow":[{"blur":1001,"color":"#000"}]}),
        json!({"fill":{"type":"linear","stops":[[0.8,"#fff"],[0.2,"#000"]]}}),
        json!({"stroke":{"color":"blue"}}),
        json!({"stroke":{"color":[],"width":1}}),
    ] {
        assert!(
            compute(&input(props.clone()), &MeasureCache::new()).is_err(),
            "{props}"
        );
    }
}
#[test]
fn legacy_solid_scalar_and_empty_shadows_keep_old_shapes() {
    let LayoutStep::Ready { scene } = compute(
        &input(json!({"fill":"#fff","radius":7,"shadow":[]})),
        &MeasureCache::new(),
    )
    .unwrap() else {
        panic!()
    };
    let n = &serde_json::to_value(scene).unwrap()["nodes"][0];
    assert_eq!(n["radius"], 7.);
    assert_eq!(n["fill"], "#fff");
    assert!(n.get("shadow").is_none());
}
/// Rejected at the input boundary (serde) or by validation; never reaches the scene.
fn rejected(kind: &str, props: Value) -> bool {
    let raw = json!({"schemaVersion":1,"projectId":"p","revision":1,"frameId":"f","viewport":{"width":200,"height":120},"nodes":[
        {"id":"f","parentId":null,"kind":"frame","props":{}},
        {"id":"n","parentId":"f","kind":kind,"props":props}],
        "fonts":[{"id":"font_bundled","family":"Noto Sans","hash":"h","url":"/font.ttf"}]});
    serde_json::from_value::<LayoutInput>(raw)
        .map_or(true, |i| compute(&i, &MeasureCache::new()).is_err())
}
#[test]
fn direct_inputs_cannot_bypass_storage_paint_rules() {
    let stops = |n: usize| {
        (0..n)
            .map(|i| json!([i as f64 / n as f64, "#000"]))
            .collect::<Vec<_>>()
    };
    let nine = (0..9).map(|_| json!({"color":"#000"})).collect::<Vec<_>>();
    for (kind, props) in [
        ("rectangle", json!({"radius":[1,2,3,4,5]})),
        ("rectangle", json!({"radius":{"topLeft":4}})),
        ("rectangle", json!({"radius":100001})),
        (
            "rectangle",
            json!({"fill":{"type":"conic","stops":stops(2)}}),
        ),
        (
            "rectangle",
            json!({"fill":{"type":"linear","angle":361,"stops":stops(2)}}),
        ),
        (
            "rectangle",
            json!({"fill":{"type":"linear","stops":stops(1)}}),
        ),
        (
            "rectangle",
            json!({"fill":{"type":"linear","stops":stops(17)}}),
        ),
        (
            "rectangle",
            json!({"fill":{"type":"linear","stops":[[0,"#000"],[1.5,"#fff"]]}}),
        ),
        (
            "rectangle",
            json!({"fill":{"type":"linear","stops":[[0,"#000"],[1,"red"]]}}),
        ),
        (
            "rectangle",
            json!({"fill":{"type":"linear","center":[0,0],"stops":stops(2)}}),
        ),
        (
            "rectangle",
            json!({"fill":{"type":"radial","radius":[0,4],"stops":stops(2)}}),
        ),
        (
            "rectangle",
            json!({"fill":{"type":"radial","center":[11,0],"stops":stops(2)}}),
        ),
        (
            "text",
            json!({"text":"x","fontId":"font_bundled","fontSize":12,"lineHeight":16,"fill":{"type":"linear","stops":stops(2)}}),
        ),
        (
            "text",
            json!({"text":"x","fontId":"font_bundled","fontSize":12,"lineHeight":16,"shadow":[{"color":"#000"}]}),
        ),
        ("rectangle", json!({"shadow":nine})),
        (
            "rectangle",
            json!({"shadow":[{"color":"#000","offsetX":1}]}),
        ),
        (
            "rectangle",
            json!({"shadow":[{"color":"#000","spread":-1001}]}),
        ),
        ("rectangle", json!({"shadow":[{"y":1}]})),
        ("rectangle", json!({"shadow":[{"color":"rgba(0,0,0,.5)"}]})),
        (
            "rectangle",
            json!({"stroke":{"color":"#000","width":[1,2,3,4,5]}}),
        ),
    ] {
        assert!(rejected(kind, props.clone()), "{kind} {props}");
    }
    // Controls: the same shapes pass when valid, so each rejection above is the paint rule.
    assert!(!rejected(
        "text",
        json!({"text":"x","fontId":"font_bundled","fontSize":12,"lineHeight":16,"fill":"#000"})
    ));
    assert!(!rejected(
        "rectangle",
        json!({"radius":[1,2,3,4],"fill":{"type":"radial","radius":[2,4],"center":[10,0],"stops":stops(16)},"shadow":(0..8).map(|_| json!({"color":"#000","spread":-1000})).collect::<Vec<_>>()})
    ));
}
#[test]
fn css_shorthand_expands_one_to_four_values_for_radius_and_stroke() {
    for (short, full) in [
        (json!([5]), json!([5., 5., 5., 5.])),
        (json!([5, 6]), json!([5., 6., 5., 6.])),
        (json!([5, 6, 7]), json!([5., 6., 7., 6.])),
        (json!([5, 6, 7, 8]), json!([5., 6., 7., 8.])),
    ] {
        let i = input(json!({"radius":short,"stroke":{"color":"#000","width":short}}));
        let LayoutStep::Ready { scene } = compute(&i, &MeasureCache::new()).unwrap() else {
            panic!()
        };
        let n = &serde_json::to_value(scene).unwrap()["nodes"][0];
        assert_eq!(n["radius"], full);
        assert_eq!(n["stroke"]["width"], full);
    }
    let LayoutStep::Ready{scene} = compute(&input(json!({"fill":{"type":"linear","stops":[[0,"#fff"],[0.5,"#0008"],[0.5,"#000"],[1,"#0000"]]}})), &MeasureCache::new()).unwrap() else { panic!() };
    let n = &serde_json::to_value(scene).unwrap()["nodes"][0];
    assert_eq!(
        n["fill"],
        json!({"type":"linear","angle":180.,"stops":[[0.,"#fff"],[0.5,"#0008"],[0.5,"#000"],[1.,"#0000"]]}),
        "hard stops keep order"
    );
}
