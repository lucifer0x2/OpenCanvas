//! Opt-in dev fixture: captured font bytes are ignored, never bundled or runtime I/O.
use opencanvas_text::{Font, FontSet, MeasureRequest};
use serde_json::Value;
use std::{fs, path::PathBuf};

fn exact(actual: &Value, expected: &Value, label: &str) {
    match (actual.as_f64(), expected.as_f64()) {
        (Some(a), Some(e)) => assert_eq!(a, e, "{label}"),
        _ => assert_eq!(actual, expected, "{label}"),
    }
}

#[test]
#[ignore = "requires exact G1 out/oracle.json and out/fonts capture"]
fn captured_inter_dot_attaches_in_latin_item_with_unchanged_measurement() {
    let out =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spikes/native-render-parity/out");
    let capture: Value =
        serde_json::from_slice(&fs::read(out.join("oracle.json")).unwrap()).unwrap();
    let face = capture["fonts"]
        .as_array()
        .unwrap()
        .iter()
        .find(|f| f["manifest"]["id"] == "inter")
        .unwrap();
    let m = &face["manifest"];
    let font = Font::from_bytes(
        fs::read(out.join(face["sourceBytes"].as_str().unwrap())).unwrap(),
        m["hash"].as_str().unwrap(),
    )
    .unwrap()
    .with_face(
        m["family"].as_str().unwrap(),
        m["weight"].as_u64().unwrap() as u16,
        m["style"].as_str().unwrap(),
    )
    .unwrap();
    let run = font.positioned_run("ς i\u{307}", 16.0, 0.0).unwrap();
    assert_eq!(run.advance, 17.282150268554688);
    assert_eq!(
        run.glyphs.iter().map(|g| g.cluster).collect::<Vec<_>>(),
        [0, 2, 3, 3]
    );
    assert_eq!(run.glyphs[2].x, 13.47137451171875);
    assert_eq!(run.glyphs[3].x, 13.484390258789063);
    assert_eq!(run.glyphs[3].x_advance, 0.0);
    assert_eq!(run.glyphs[3].y, 0.0);
    let ink = font.ink_bounds(&run, 16.0).unwrap();
    assert_eq!(ink.width as f64, 15.574670791625977);
    let expected = &capture["browser"]["measurements"]["current/inter/86"];
    assert_eq!(expected["lines"][5]["text"], "ς i\u{307}");
    assert_eq!(
        expected["lines"][5]["inkBounds"]["width"],
        15.574615478515625
    );
    // Keep the remaining strict ink discrepancy visible; do not clamp to Canvas.
    assert_ne!(
        ink.width as f64,
        expected["lines"][5]["inkBounds"]["width"].as_f64().unwrap()
    );
    let mut set = FontSet::new();
    set.insert("inter", font).unwrap();
    for key in [
        "current/inter/86",
        "current/inter/87",
        "current/inter/84/ceil64-below",
    ] {
        let request: MeasureRequest = serde_json::from_value(
            capture["requests"]
                .as_array()
                .unwrap()
                .iter()
                .find(|r| r["key"] == key)
                .unwrap()
                .clone(),
        )
        .unwrap();
        let actual = serde_json::to_value(set.measure(&request).unwrap()).unwrap();
        let expected = &capture["browser"]["measurements"][key];
        for field in [
            "width",
            "height",
            "ascent",
            "missing",
            "coverageUnavailable",
        ] {
            exact(&actual[field], &expected[field], &format!("{key}/{field}"));
        }
        for (a, e) in actual["lines"]
            .as_array()
            .unwrap()
            .iter()
            .zip(expected["lines"].as_array().unwrap())
        {
            for field in ["text", "x", "y", "width"] {
                exact(&a[field], &e[field], &format!("{key}/{field}"));
            }
        }
        assert_eq!(
            actual["lines"].as_array().unwrap().len(),
            expected["lines"].as_array().unwrap().len()
        );
    }
}
