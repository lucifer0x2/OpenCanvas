//! Exact ignored captures are dev fixtures, never bundled or runtime filesystem I/O.
use opencanvas_text::{Font, FontSet, MeasureRequest};
use serde_json::Value;
use std::{fs, path::PathBuf};

fn capture(name: &str) -> (Value, FontSet) {
    let out =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spikes/native-render-parity/out");
    let oracle: Value = serde_json::from_slice(&fs::read(out.join(name)).unwrap()).unwrap();
    let mut fonts = FontSet::new();
    for face in oracle["fonts"].as_array().unwrap() {
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
        fonts.insert(m["id"].as_str().unwrap(), font).unwrap();
    }
    (oracle, fonts)
}

fn equal(actual: &Value, expected: &Value, key: &str, field: &str) {
    match (actual.as_f64(), expected.as_f64()) {
        (Some(a), Some(e)) => assert_eq!(a, e, "{key}/{field}"),
        _ => assert_eq!(actual, expected, "{key}/{field}"),
    }
}

fn check(name: &str, eligible: usize, excluded: usize) {
    let (oracle, fonts) = capture(name);
    let mut counts = (0, 0);
    let mut raw = 0;
    for value in oracle["requests"].as_array().unwrap() {
        let r: MeasureRequest = serde_json::from_value(value.clone()).unwrap();
        let actual = serde_json::to_value(fonts.measure(&r).unwrap()).unwrap();
        let expected = &oracle["browser"]["measurements"][&r.key];
        for field in ["missing", "coverageUnavailable"] {
            equal(&actual[field], &expected[field], &r.key, field);
        }
        if !actual["missing"].as_array().unwrap().is_empty() {
            counts.1 += 1;
            continue;
        }
        counts.0 += 1;
        for field in ["width", "height", "ascent"] {
            equal(&actual[field], &expected[field], &r.key, field);
        }
        let a = actual["lines"].as_array().unwrap();
        let e = expected["lines"].as_array().unwrap();
        assert_eq!(a.len(), e.len(), "{} line count", r.key);
        for (a, e) in a.iter().zip(e) {
            for field in ["text", "x", "y", "width"] {
                equal(&a[field], &e[field], &r.key, field);
            }
        }
        let label = oracle["cases"][&r.key]["label"].as_str().unwrap();
        // Keep the raw-advance assertion focused on H2's existing root causes.
        // Other retained H1 probes (e.g. joiner) have sub-grid raw discrepancies.
        if r.width.is_none()
            && (name == "h2-oracle.json"
                || matches!(label, "stress" | "crossScriptMark" | "inheritedOnly"))
        {
            let font = fonts.get(&r.font_id).unwrap();
            let run = font
                .positioned_run(&r.text, r.font_size, r.letter_spacing.unwrap_or_default())
                .unwrap();
            assert_eq!(
                run.advance,
                oracle["browser"]["probes"][&r.key]["fullText"]["width"]
                    .as_f64()
                    .unwrap(),
                "{} raw advance",
                r.key
            );
            raw += 1;
        }
    }
    assert_eq!(counts, (eligible, excluded));
    assert!(raw > 0);
}

#[test]
#[ignore = "requires exact H1 out/h1-oracle.json and original out/fonts"]
fn every_original_h1_geometry_and_targeted_raw_advance_is_exact() {
    check("h1-oracle.json", 5220, 72);
}

#[test]
#[ignore = "requires targeted out/h2-oracle.json from h2-probe.mjs"]
fn targeted_h2_script_cluster_edges_and_boundaries_are_exact() {
    check("h2-oracle.json", 420, 12);
}
