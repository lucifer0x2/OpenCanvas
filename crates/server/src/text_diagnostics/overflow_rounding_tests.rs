//! Measured U07 Linux ink extents and controls for the own-box advisory policy.
use super::append;
use serde_json::{json, Value};

fn snug(width: f64, ink_x: f64, ink_width: f64) -> Value {
    json!({"diagnostics":[],"provenance":{"sourceSceneKey":"rounding"},"nodes":[
        {"id":"frame","kind":"frame","bounds":{"x":0,"y":0,"width":400,"height":80}},
        {"id":"label","parentId":"frame","kind":"text","fontId":"font",
         "bounds":{"x":10,"y":10,"width":width,"height":20},
         "lines":[{"x":0,"y":14,"width":width,
             "inkBounds":{"x":ink_x,"y":-10,"width":ink_width,"height":12}}]}]})
}

fn example(value: &mut Value) -> Value {
    append(value, &[]);
    assert_eq!(value["diagnostics"][0]["code"], "TEXT_CONTENT_OUTSIDE_BOX");
    value["diagnostics"][0]["examples"][0].clone()
}

#[test]
fn ten_u07_snug_advance_boxes_allow_bounded_horizontal_ink_overhang() {
    // No authored strings or resources are needed: diagnostics only use measured geometry.
    for (name, width, ink_x, ink_width) in [
        ("title", 241.09375, -1., 241.08839416503903),
        ("sc-primary-l5-x", 49.421875, 0., 50.12030029296875),
        ("sc-secondary-l4-x", 48.890625, 0., 49.59230041503906),
        ("sc-accent-l0-x", 48.0625, 0., 48.7523193359375),
        ("sc-accent-l6-x", 47.265625, 0., 47.960296630859375),
        ("sc-accent-l9-x", 47.265625, 0., 47.960296630859375),
        ("sc-neutral-l0-x", 46.609375, 0., 47.31231689453125),
        ("sem-danger-l2-x", 51.09375, 0., 51.620330810546875),
        ("tok0-x", 46.609375, 0., 47.31231689453125),
        ("bn-success-ig", 9.40625, 0., 10.),
    ] {
        let mut value = snug(width, ink_x, ink_width);
        let nodes = value["nodes"].clone();
        append(&mut value, &[]);
        assert_eq!(value["diagnostics"], json!([]), "{name}");
        assert_eq!(value["nodes"], nodes, "advisory must not change geometry");
    }
}

#[test]
fn genuine_advance_and_significant_ink_overflow_still_warn() {
    let mut advance = snug(20., 0., 20.);
    advance["nodes"][1]["lines"][0]["width"] = json!(20.6);
    assert_eq!(example(&mut advance)["advance"]["outsideOwnBox"], true);
    for (x, width) in [(-1.01, 20.), (0., 21.01)] {
        let mut value = snug(20., x, width);
        assert_eq!(example(&mut value)["ink"]["outsideOwnBox"], true);
    }
}

#[test]
fn allowance_requires_both_advance_edges_snug_to_own_box() {
    let mut shorter = snug(20., -0.6, 20.);
    shorter["nodes"][1]["lines"][0]["width"] = json!(19.5);
    assert_eq!(example(&mut shorter)["ink"]["outsideOwnBox"], true);
    let mut shifted = snug(20., -1.1, 20.);
    shifted["nodes"][1]["lines"][0]["x"] = json!(0.5);
    assert_eq!(example(&mut shifted)["ink"]["outsideOwnBox"], true);
}

#[test]
fn vertical_and_subpixel_own_or_ancestor_clipping_remain_evidence() {
    for (ink_y, ink_height) in [(-14.6, 12.), (-10., 16.6)] {
        let mut value = snug(20., 0., 20.);
        value["nodes"][1]["lines"][0]["inkBounds"]["y"] = json!(ink_y);
        value["nodes"][1]["lines"][0]["inkBounds"]["height"] = json!(ink_height);
        assert_eq!(example(&mut value)["ink"]["outsideOwnBox"], true);
    }
    let mut own = snug(20., 0., 20.6);
    own["nodes"][1]["clip"] = json!(true);
    let evidence = example(&mut own);
    assert_eq!(evidence["ink"]["outsideOwnBox"], false);
    assert_eq!(evidence["clipNodeIds"], json!(["label"]));
    let mut ancestor = snug(20., 0., 20.6);
    ancestor["nodes"][0]["clip"] = json!(true);
    ancestor["nodes"][0]["bounds"] = ancestor["nodes"][1]["bounds"].clone();
    assert_eq!(example(&mut ancestor)["clipNodeIds"], json!(["frame"]));
}

#[test]
fn missing_ink_and_hidden_nodes_keep_their_existing_behavior() {
    let mut absent = snug(20., 0., 20.6);
    absent["nodes"][1]["lines"][0]["inkBounds"] = Value::Null;
    append(&mut absent, &[]);
    assert_eq!(absent["diagnostics"], json!([]));
    let mut hidden = snug(20., 0., 22.);
    hidden["nodes"][1]["opacity"] = json!(0);
    append(&mut hidden, &[]);
    assert_eq!(hidden["diagnostics"], json!([]));
}
