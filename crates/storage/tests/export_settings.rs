mod common;
use common::*;
use opencanvas_storage::*;
use serde_json::{json, Value};

fn update(id: &str, settings: Value) -> Operation {
    Operation::UpdateNode {
        id: id.into(),
        props: json!({ "exportSettings": settings }),
    }
}

#[test]
fn settings_round_trip_and_empty_array_clears() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let settings = json!([{"scale":1},{"scale":2},{"width":96,"suffix":"-96"}]);
    p.apply_batch(request(
        &p,
        1,
        "mark",
        vec![update("rect", settings.clone())],
    ))
    .unwrap();
    let snapshot = p.snapshot().unwrap();
    let rect = snapshot.nodes.iter().find(|n| n.id == "rect").unwrap();
    assert_eq!(rect.props["exportSettings"], settings);
    p.apply_batch(request(&p, 2, "clear", vec![update("rect", json!([]))]))
        .unwrap();
    let snapshot = p.snapshot().unwrap();
    let rect = snapshot.nodes.iter().find(|n| n.id == "rect").unwrap();
    assert_eq!(rect.props["exportSettings"], json!([]));
}

#[test]
fn invalid_settings_name_the_failing_item_and_key() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let nine: Vec<Value> = (0..9).map(|i| json!({"suffix": format!("-{i}")})).collect();
    let cases = [
        (json!([{"scale":0}]), "exportSettings[0].scale"),
        (json!([{"scale":1},{"scale":5}]), "exportSettings[1].scale"),
        (json!([{"scale":2,"width":10}]), "exportSettings[0]"),
        (json!([{"format":"pdf"}]), "exportSettings[0].format"),
        (
            json!([{"format":"svg","scale":2}]),
            "exportSettings[0].scale",
        ),
        (
            json!([{"format":"svg","width":24}]),
            "exportSettings[0].width",
        ),
        (
            json!([{"format":"svg","svgIdAttribute":"yes"}]),
            "exportSettings[0].svgIdAttribute",
        ),
        (
            json!([{"svgIdAttribute":true}]),
            "exportSettings[0].svgIdAttribute",
        ),
        (
            json!([{"format":"svg"},{"format":"svg"}]),
            "exportSettings[1]",
        ),
        (
            json!([{"constraint":{"type":"SCALE","value":2}}]),
            "exportSettings[0].constraint",
        ),
        (json!([{"scale":2},{"scale":2}]), "exportSettings[1]"),
        (json!([{"scale":1},{"width":96}]), "exportSettings[1]"),
        (Value::Array(nine), "exportSettings"),
        (json!([{"suffix":"a/b"}]), "exportSettings[0].suffix"),
        (json!([{"width":1.5}]), "exportSettings[0].width"),
        (json!({"scale":2}), "exportSettings"),
    ];
    for (index, (settings, field)) in cases.into_iter().enumerate() {
        let key = format!("bad-{index}");
        let error = p
            .apply_batch(request(&p, 1, &key, vec![update("rect", settings.clone())]))
            .expect_err(&format!("{settings} must be rejected"));
        assert_eq!(error.code, "VALIDATION_FAILED", "{settings}");
        assert_eq!(error.field.as_deref(), Some(field), "{settings}: {error:?}");
    }
    let hint = p
        .apply_batch(request(
            &p,
            1,
            "type",
            vec![update("rect", json!([{"type":"png"}]))],
        ))
        .unwrap_err();
    assert!(hint.message.contains("format"), "{}", hint.message);
}

#[test]
fn pages_cannot_carry_export_settings() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let error = p
        .apply_batch(request(
            &p,
            1,
            "page",
            vec![update("page", json!([{"scale":2}]))],
        ))
        .unwrap_err();
    assert_eq!(error.code, "VALIDATION_FAILED");
    assert_eq!(error.field.as_deref(), Some("props.exportSettings"));
}

#[test]
fn suffix_defaults_follow_the_scale() {
    let suffix = |v: Value| export_suffix(v.as_object().unwrap());
    assert_eq!(suffix(json!({})), "");
    assert_eq!(suffix(json!({"scale":1})), "");
    assert_eq!(suffix(json!({"scale":2})), "@2x");
    assert_eq!(suffix(json!({"scale":1.5})), "@1.5x");
    assert_eq!(suffix(json!({"width":96})), "");
    assert_eq!(suffix(json!({"scale":3,"suffix":"-hi"})), "-hi");
    assert!(check_export_settings(&json!([{"scale":2},{"scale":3}]), "settings").is_ok());
}

#[test]
fn svg_settings_sit_beside_png_ones() {
    let (_temp, _store, p) = setup();
    p.apply_batch(request(&p, 0, "initial", initial())).unwrap();
    let cases = [
        json!([{"format":"svg"}]),
        json!([{"format":"svg","svgIdAttribute":true},{"scale":2}]),
        json!([{"format":"svg"},{"scale":1}]),
    ];
    for (index, settings) in cases.into_iter().enumerate() {
        let key = format!("svg-{index}");
        p.apply_batch(request(
            &p,
            index as u64 + 1,
            &key,
            vec![update("rect", settings)],
        ))
        .unwrap();
    }
    let error =
        check_export_settings(&json!([{"format":"svg","scale":2}]), "settings").unwrap_err();
    assert!(
        error.message.contains("resolution-independent"),
        "{}",
        error.message
    );
}
