//! Optional exact G1/V1 local fixtures; no runtime I/O or redistributed binaries.
#[path = "support/outlines.rs"]
mod outlines;
use opencanvas_text::{Font, FontSet, MeasureRequest};
use serde_json::Value;
use std::{fs, path::PathBuf};

fn captured() -> (Value, FontSet) {
    let out =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../spikes/native-render-parity/out");
    let oracle: Value =
        serde_json::from_slice(&fs::read(out.join("v1-oracle.json")).unwrap()).unwrap();
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

fn exact(actual: &Value, expected: &Value, key: &str, field: &str) {
    match (actual.as_f64(), expected.as_f64()) {
        (Some(a), Some(e)) => assert_eq!(a, e, "{key}/{field}"),
        _ => assert_eq!(actual, expected, "{key}/{field}"),
    }
}

#[test]
#[ignore = "requires v1-capture.mjs and original G1 out/fonts"]
fn all_bounded_variable_geometries_and_font_bbox_metrics_are_exact() {
    let (oracle, fonts) = captured();
    let mut count = 0;
    for value in oracle["requests"].as_array().unwrap() {
        let r: MeasureRequest = serde_json::from_value(value.clone()).unwrap();
        let actual = serde_json::to_value(fonts.measure(&r).unwrap()).unwrap();
        let expected = &oracle["browser"]["measurements"][&r.key];
        for field in [
            "width",
            "height",
            "ascent",
            "missing",
            "coverageUnavailable",
        ] {
            exact(&actual[field], &expected[field], &r.key, field);
        }
        let actual_lines = actual["lines"].as_array().unwrap();
        let expected_lines = expected["lines"].as_array().unwrap();
        assert_eq!(actual_lines.len(), expected_lines.len(), "{} lines", r.key);
        for (a, e) in actual_lines.iter().zip(expected_lines) {
            for field in ["text", "x", "y", "width"] {
                exact(&a[field], &e[field], &r.key, field);
            }
        }
        let metrics = fonts.get(&r.font_id).unwrap().metrics(r.font_size).unwrap();
        let empty = &oracle["browser"]["probes"][&r.key]["empty"];
        assert_eq!(
            metrics.ascent as f64,
            empty["fontBoundingBoxAscent"].as_f64().unwrap()
        );
        assert_eq!(
            metrics.descent as f64,
            empty["fontBoundingBoxDescent"].as_f64().unwrap()
        );
        count += 1;
    }
    assert_eq!(count, 567);
}

#[test]
#[ignore = "requires pinned original Inter bytes from G1"]
fn weight_mapping_and_outlines_survive_every_optical_instance() {
    let (_, fonts) = captured();
    for size in [16.0, 24.0, 32.0] {
        let mut paths = Vec::new();
        for (weight, coordinate) in [
            (100, -16384),
            (400, 0),
            (500, 2949),
            (700, 8847),
            (900, 16384),
        ] {
            let font = fonts.get(&format!("v1-inter-{weight}")).unwrap();
            let instance = font.shaping_font_at_size(size).unwrap();
            assert_eq!(instance.normalized_coords()[1].to_bits(), coordinate);
            assert_eq!(
                instance.normalized_coords()[0].to_bits(),
                match size as u32 {
                    16 => 1820,
                    24 => 9102,
                    _ => 16384,
                }
            );
            paths.push(outlines::outlines(font, size, "AVAg"));
        }
        for pair in paths.windows(2) {
            assert_ne!(pair[0], pair[1], "real outlines must vary at {size}px");
        }
        for weight in [400, 700] {
            let normal = fonts.get(&format!("v1-inter-{weight}")).unwrap();
            let alias = fonts.get(&format!("v1-inter-{weight}-italic")).unwrap();
            assert_eq!(
                outlines::outlines(normal, size, "AVAg"),
                outlines::outlines(alias, size, "AVAg")
            );
        }
    }
}
