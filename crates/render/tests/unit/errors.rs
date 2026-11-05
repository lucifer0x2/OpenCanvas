use super::*;
use crate::{RenderOptions, ResourceBytes};
use std::cell::Cell;

#[test]
fn transparent_background_scale_ceil_and_limits() {
    let mut s = scene(vec![]);
    s.provenance.viewport.width = 31.2;
    s.provenance.viewport.height = 12.1;
    let o = RenderOptions {
        scale: 1.5,
        ..RenderOptions::default()
    };
    let r = crate::render(&s, &Resources::default(), &o).unwrap();
    assert_eq!((r.width, r.height), (47, 19));
    assert!(r.rgba.iter().all(|v| *v == 0));
    for scale in [0.0, -1.0, 4.001, f64::NAN, f64::INFINITY] {
        let o = RenderOptions {
            scale,
            ..RenderOptions::default()
        };
        assert_eq!(
            crate::render(&s, &Resources::default(), &o)
                .unwrap_err()
                .code,
            "PIXEL_BUDGET"
        );
    }
    s.provenance.viewport.width = 4000.1;
    s.provenance.viewport.height = 4000.0;
    assert_eq!(
        crate::render(&s, &Resources::default(), &RenderOptions::default())
            .unwrap_err()
            .code,
        "PIXEL_BUDGET"
    );
}
#[test]
fn version_and_both_resource_hash_mismatches() {
    let mut s = scene(vec![]);
    s.provenance.renderer_version = "old".into();
    assert_eq!(
        crate::render(&s, &Resources::default(), &RenderOptions::default())
            .unwrap_err()
            .code,
        "PIPELINE_MISMATCH"
    );
    s.provenance.renderer_version = "web-4".into();
    s.fonts.push(
        serde_json::from_value(json!({"id":"f","family":"Test","hash":"00","url":"never-fetch"}))
            .unwrap(),
    );
    let mut r = Resources::default();
    r.fonts.insert(
        "f".into(),
        ResourceBytes {
            hash: "00",
            bytes: b"invalid font",
        },
    );
    assert_eq!(
        crate::render(&s, &r, &RenderOptions::default())
            .unwrap_err()
            .code,
        "PIPELINE_MISMATCH"
    );
    s.fonts.clear();
    r.fonts.clear();
    s.assets.push(
        serde_json::from_value(
            json!({"id":"a","hash":"00","url":"never-fetch","mediaType":"image/png"}),
        )
        .unwrap(),
    );
    r.assets.insert(
        "a".into(),
        ResourceBytes {
            hash: "wrong",
            bytes: b"invalid image",
        },
    );
    assert_eq!(
        crate::render(&s, &r, &RenderOptions::default())
            .unwrap_err()
            .code,
        "PIPELINE_MISMATCH"
    );
}
#[test]
fn strict_and_recognizable_non_strict_placeholders() {
    for kind in ["text", "image"] {
        let mut n = node("missing", 1.0, 1.0, 62.0, 30.0, "transparent");
        n.kind = kind.into();
        let s = scene(vec![n]);
        assert_eq!(
            crate::render(&s, &Resources::default(), &RenderOptions::default())
                .unwrap_err()
                .code,
            "RESOURCE_NOT_READY"
        );
        let o = RenderOptions {
            strict: false,
            ..RenderOptions::default()
        };
        let r = crate::render(&s, &Resources::default(), &o).unwrap();
        assert!(
            r.rgba
                .as_chunks::<4>()
                .0
                .iter()
                .filter(|p| p[3] > 0)
                .count()
                > 30
        );
        assert!(r
            .diagnostics
            .iter()
            .any(|d| d["code"] == "PLACEHOLDER_BITMAP_LABEL"));
    }
}
#[test]
fn unused_manifest_missing_bytes_still_fail_strict_readiness() {
    let mut s = scene(vec![]);
    s.assets.push(
        serde_json::from_value(
            json!({"id":"unused","hash":"00","url":"unused","mediaType":"image/png"}),
        )
        .unwrap(),
    );
    assert_eq!(
        crate::render(&s, &Resources::default(), &RenderOptions::default())
            .unwrap_err()
            .code,
        "RESOURCE_NOT_READY"
    );
    let r = crate::render(
        &s,
        &Resources::default(),
        &RenderOptions {
            strict: false,
            ..RenderOptions::default()
        },
    )
    .unwrap();
    assert_eq!(r.readiness[0].status, "failed");
    assert_eq!(r.readiness[0].kind, "asset");
}
#[test]
fn opaque_diagnostics_and_unknown_runtime_provenance_round_trip() {
    let s = scene(vec![]);
    let mut v = serde_json::to_value(s).unwrap();
    v["diagnostics"] = json!([{"code":"MISSING_GLYPH","nested":[1,2]},"legacy"]);
    v["provenance"]["runtimeVersion"] = json!("native-test");
    let s: PaintScene = serde_json::from_value(v.clone()).unwrap();
    assert_eq!(serde_json::to_value(&s).unwrap(), v);
    // Scene advisory objects are not resource readiness failures.
    let raster = crate::render(&s, &Resources::default(), &RenderOptions::default()).unwrap();
    assert_eq!(raster.diagnostics, s.diagnostics);
}
#[test]
fn cancellation_checks_during_traversal_and_blur() {
    let count = Cell::new(0);
    let cancel = || {
        count.set(count.get() + 1);
        count.get() > 8
    };
    let nodes = (0..12)
        .map(|i| node(&format!("n{i}"), 1.0, 1.0, 30.0, 30.0, "#f00"))
        .collect();
    let o = RenderOptions {
        cancelled: Some(&cancel),
        ..RenderOptions::default()
    };
    assert_eq!(
        crate::render(&scene(nodes), &Resources::default(), &o)
            .unwrap_err()
            .code,
        "CANCELLED"
    );
    let cancel = || true;
    let o = RenderOptions {
        cancelled: Some(&cancel),
        ..RenderOptions::default()
    };
    assert_eq!(
        crate::blur::blur(&mut [255; 64], 8, 8, 4.0, &o)
            .unwrap_err()
            .code,
        "CANCELLED"
    );
}
#[test]
fn cycles_duplicates_and_malformed_geometry_fail_without_panicking() {
    let mut n = node("n", 0.0, 0.0, 10.0, 10.0, "#f00");
    let s = scene(vec![n.clone(), n.clone()]);
    assert_eq!(
        crate::render(&s, &Resources::default(), &RenderOptions::default())
            .unwrap_err()
            .code,
        "SCENE_INVALID"
    );
    n.parent_id = Some("n".into());
    assert!(crate::render(
        &scene(vec![n.clone()]),
        &Resources::default(),
        &RenderOptions::default()
    )
    .is_err());
    n.parent_id = None;
    n.radius = opencanvas_layout::Sides::Short(vec![]);
    assert!(crate::render(
        &scene(vec![n]),
        &Resources::default(),
        &RenderOptions::default()
    )
    .is_err());
    assert!(crate::color::color("#ééé").is_err());
}
