use super::*;
use serde_json::{json, Value};

fn scene(ink: Value) -> Value {
    json!({"diagnostics":[],"provenance":{"sourceSceneKey":"key","viewport":{"width":100,"height":80},"fontManifestHash":"hash"},"nodes":[
        {"id":"frame","parentId":null,"kind":"frame","clip":true,"bounds":{"x":0,"y":0,"width":100,"height":80}},
        {"id":"label","parentId":"frame","kind":"text","fontId":"font","bounds":{"x":10,"y":10,"width":20,"height":10},"lines":[{"x":0,"y":8,"width":35,"inkBounds":ink}]}]})
}

#[test]
fn advance_and_ink_are_separate_bounded_measured_evidence() {
    let mut value = scene(json!({"x":-2,"y":-9,"width":18,"height":12}));
    append(&mut value, &[]);
    let d = &value["diagnostics"][0];
    assert_eq!(d["code"], "TEXT_CONTENT_OUTSIDE_BOX");
    assert_eq!(d["examples"][0]["advance"]["width"].as_f64(), Some(35.));
    assert_eq!(d["examples"][0]["ink"]["width"].as_f64(), Some(18.));
    assert_eq!(d["inkEvidence"], "measured");
    assert_eq!(d["provenance"]["sourceSceneKey"], "key");
    assert!(!d.to_string().contains("text\":"));
    let mut absent = scene(Value::Null);
    append(&mut absent, &["label".into()]);
    assert_eq!(absent["diagnostics"][0]["inkEvidence"], "unavailable");
    assert_eq!(absent["diagnostics"][0]["severity"], "info");
    assert_eq!(absent["diagnostics"][0]["intentionalOverflow"], true);
}

#[test]
fn fitting_advance_can_have_ink_overhang_or_be_clean() {
    let mut clean = scene(json!({"x":0,"y":-7,"width":15,"height":8}));
    clean["nodes"][1]["lines"][0]["width"] = json!(15);
    append(&mut clean, &[]);
    assert_eq!(clean["diagnostics"], json!([]));
    clean["nodes"][1]["lines"][0]["inkBounds"]["x"] = json!(-2);
    append(&mut clean, &[]);
    let evidence = &clean["diagnostics"][0]["examples"][0];
    assert_eq!(evidence["advance"]["outsideOwnBox"], false);
    assert_eq!(evidence["ink"]["outsideOwnBox"], true);
}

#[test]
fn ancestor_clip_is_evidence_even_when_own_box_fits_and_hidden_text_is_ignored() {
    let mut value = scene(json!({"x":0,"y":-7,"width":15,"height":8}));
    value["nodes"][1]["bounds"] = json!({"x":80,"y":10,"width":100,"height":100});
    append(&mut value, &[]);
    let example = &value["diagnostics"][0]["examples"][0];
    assert_eq!(example["advance"]["outsideOwnBox"], false);
    assert_eq!(example["ink"]["outsideOwnBox"], false);
    assert_eq!(example["clipNodeIds"], json!(["frame"]));
    value["diagnostics"] = json!([]);
    value["nodes"][0]["opacity"] = json!(0);
    append(&mut value, &[]);
    assert_eq!(value["diagnostics"], json!([]));
}

#[test]
fn many_overflows_have_bounded_examples_and_an_explicit_omission_summary() {
    let mut value = scene(Value::Null);
    let mut template = value["nodes"][1].clone();
    template["lines"] = json!(vec![template["lines"][0].clone(); 4]);
    for i in 0..14 {
        template["id"] = json!(format!("text-{i}"));
        value["nodes"]
            .as_array_mut()
            .unwrap()
            .push(template.clone());
    }
    append(&mut value, &[]);
    let issues = value["diagnostics"].as_array().unwrap();
    assert_eq!(issues.len(), 13);
    assert_eq!(issues[1]["outsideLineCount"], 4);
    assert_eq!(issues[1]["examples"].as_array().unwrap().len(), 2);
    assert_eq!(issues[12]["code"], "TEXT_OVERFLOW_SUMMARY");
    assert_eq!(issues[12]["omittedNodes"], 3);
}
