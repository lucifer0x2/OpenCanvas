use super::*;
use serde_json::{Value, json};

fn fixture() -> Value {
    json!({"schemaVersion":1,"projectId":"p","revision":1,"frameId":"f",
        "viewport":{"width":390,"height":844},"nodes":[
        {"id":"f","kind":"frame","props":{"layout":{"mode":"column"}}},
        {"id":"row","parentId":"f","kind":"group","props":{
            "width":"fill","height":"hug","layout":{"mode":"row","wrap":true}}},
        {"id":"card","parentId":"row","kind":"group","props":{
            "width":"fill","minWidth":280,"height":80}}]})
}
fn compute_value(value: Value) -> Result<LayoutStep, LayoutError> {
    compute(
        &serde_json::from_value(value).unwrap(),
        &MeasureCache::new(),
    )
}
fn invalid(value: Value) {
    assert_eq!(compute_value(value).unwrap_err().code, "LAYOUT_INVALID");
}
#[test]
fn wrap_semantic_guards_run_before_measurement() {
    for patch in [
        json!({"layout":{"mode":"column","wrap":true}}),
        json!({"layout":{"mode":"free","wrap":true}}),
        json!({"width":"hug"}),
        json!({"width":0}),
        json!({"width":null}),
        json!({"minWidth":300,"maxWidth":200}),
        json!({"minHeight":300,"maxHeight":200}),
    ] {
        let mut v = fixture();
        v["nodes"][1]["props"]
            .as_object_mut()
            .unwrap()
            .extend(patch.as_object().unwrap().clone());
        invalid(v);
    }
    for min in [Value::Null, json!(0)] {
        let mut v = fixture();
        v["nodes"][2]["props"]["minWidth"] = min;
        // Missing font would fail later if validation were bypassed.
        v["nodes"][2]["kind"] = json!("text");
        invalid(v);
    }
    let mut v = fixture();
    v["nodes"][1]["kind"] = json!("rectangle");
    v["nodes"].as_array_mut().unwrap().pop();
    invalid(v);
}
#[test]
fn typed_boolean_and_serialization_preserve_no_wrap_shape() {
    for bad in [Value::Null, json!("true"), json!(1), json!({"tokenId":"w"})] {
        let mut v = fixture();
        v["nodes"][1]["props"]["layout"]["wrap"] = bad;
        assert!(serde_json::from_value::<LayoutInput>(v).is_err());
    }
    for mode in ["row", "column", "free"] {
        let rule: LayoutRule = serde_json::from_value(json!({"mode":mode,"wrap":false})).unwrap();
        assert!(serde_json::to_value(rule).unwrap().get("wrap").is_none());
        let mut v = fixture();
        v["nodes"][1]["props"]["layout"] = json!({"mode":mode,"wrap":false});
        assert!(compute_value(v).is_ok());
    }
}

#[test]
fn indefinite_ancestor_chain_and_no_wrap_minmax_parity() {
    let mut v = fixture();
    v["nodes"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"outer","parentId":"f","kind":"group","props":{"width":0,"height":200}}));
    v["nodes"][1]["parentId"] = json!("outer");
    invalid(v.clone());
    v["nodes"][3]["props"]["width"] = json!("hug");
    invalid(v.clone());
    v["nodes"][3]["props"]
        .as_object_mut()
        .unwrap()
        .remove("width");
    invalid(v);
    for (min, max) in [("minWidth", "maxWidth"), ("minHeight", "maxHeight")] {
        let mut v = fixture();
        v["nodes"][1]["props"]["layout"]["wrap"] = json!(false);
        v["nodes"][2]["props"][min] = json!(90);
        v["nodes"][2]["props"][max] = json!(80);
        invalid(v);
    }
    let mut v = fixture();
    v["nodes"][0]["props"]["layout"] = json!({"mode":"row","wrap":true});
    // Selected frame has an actual viewport; its immediate fill child still needs a minimum.
    v["nodes"][1]["props"]["minWidth"] = json!(280);
    assert!(compute_value(v).is_ok());
}
#[test]
fn definite_chains_root_viewport_fixed_hug_and_clamping() {
    // Selected root authored width may be absent: viewport is definite.
    assert!(matches!(
        compute_value(fixture()).unwrap(),
        LayoutStep::Ready { .. }
    ));
    for width in [json!(60), json!("hug")] {
        let mut v = fixture();
        v["nodes"][2]["props"]["width"] = width;
        v["nodes"][2]["props"]["minWidth"] = json!(80);
        v["nodes"][2]["props"]["maxWidth"] = json!(80);
        let LayoutStep::Ready { scene } = compute_value(v).unwrap() else {
            panic!()
        };
        assert_eq!(scene.nodes[2].bounds.width, 80.);
    }
    for axis in ["width", "height"] {
        let mut v = fixture();
        v["nodes"][2]["props"][axis] = json!("fill");
        v["nodes"][1]["props"][axis] = json!("hug");
        invalid(v);
    }
}
