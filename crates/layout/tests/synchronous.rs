use opencanvas_layout::*;
use serde_json::to_value;
use std::collections::BTreeSet;
mod synchronous_support;
use synchronous_support::*;

#[test]
fn synchronous_scene_equals_batched_cache_with_responsive_wrap_and_typography() {
    for width in [390.0, 400.0, 400.000_000_01, 640.0, 1040.0] {
        let input = fixture(width);
        let before = to_value(&input).unwrap();
        let mut measurer = RecordingMeasurer::default();
        let scene = compute_with_measurer(&input, &mut measurer).unwrap();
        assert_eq!(
            to_value(&scene).unwrap(),
            to_value(cached_scene(&input)).unwrap()
        );
        assert_eq!(
            to_value(&input).unwrap(),
            before,
            "authored input is immutable"
        );
        assert!(
            scene
                .diagnostics
                .iter()
                .any(|d| d.starts_with("MISSING_GLYPH"))
        );
        assert!(
            scene
                .diagnostics
                .iter()
                .any(|d| d.starts_with("GLYPH_CHECK_UNAVAILABLE"))
        );
        assert!(scene.nodes.iter().any(|n| n.decoration.is_some()));
        assert!(scene.nodes.iter().flat_map(|n| &n.lines).any(|l| l.x > 0.0));
        assert_eq!(scene.provenance.scene_version, SCENE_VERSION);
        assert_eq!(scene.provenance.renderer_version, RENDERER_VERSION);
        assert!(measurer.0.iter().any(|r| r.width.is_none()), "nowrap");
        for node in scene.nodes.iter().filter(|n| n.kind == "text") {
            let source = input.nodes.iter().find(|n| n.id == node.id).unwrap();
            let final_width = if source.props.text_wrap.as_deref() == Some("nowrap") {
                None
            } else {
                Some(node.bounds.width)
            };
            assert!(
                measurer.0.iter().any(|r| {
                    Some(&r.text) == source.props.text.as_ref() && r.width == final_width
                }),
                "final-width lines must be measured"
            );
        }
        let keys: BTreeSet<_> = measurer.0.iter().map(|r| &r.key).collect();
        assert_eq!(
            keys.len(),
            measurer.0.len(),
            "memoize identical request keys"
        );
        assert!(
            measurer
                .0
                .iter()
                .all(|r| r.width.is_none_or(|w| w * 64.0 == (w * 64.0).round()))
        );
    }
}

#[test]
fn fixed_geometry_requests_final_quantized_width_even_when_callback_is_skipped() {
    let input = fixed_text();
    let LayoutStep::Pending { requests } = compute(&input, &MeasureCache::new()).unwrap() else {
        panic!("cache adapter must not publish unmeasured final lines");
    };
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0].width, Some(37.34375));
    let mut measurer = RecordingMeasurer::default();
    let scene = compute_with_measurer(&input, &mut measurer).unwrap();
    assert_eq!(measurer.0.len(), 1);
    assert_eq!(
        to_value(&measurer.0[0]).unwrap(),
        to_value(&requests[0]).unwrap()
    );
    assert_eq!(scene.nodes[1].bounds.width, 37.34375);
    assert_eq!(
        to_value(&scene).unwrap(),
        to_value(cached_scene(&input)).unwrap()
    );
    let first_call_count = measurer.0.len();
    let repeated = compute_with_measurer(&input, &mut measurer).unwrap();
    assert_eq!(
        measurer.0.len(),
        first_call_count * 2,
        "memo is invocation-local"
    );
    assert_eq!(to_value(scene).unwrap(), to_value(repeated).unwrap());
}

struct FailingMeasurer(usize, &'static str);
impl TextMeasurer for FailingMeasurer {
    fn measure(&mut self, _: &MeasureRequest) -> Result<Measurement, LayoutError> {
        self.0 += 1;
        Err(LayoutError {
            code: self.1.into(),
            message: "synthetic provider failure".into(),
        })
    }
}

#[test]
fn provider_error_from_taffy_callback_is_not_a_false_ready_scene() {
    for code in ["JOB_TIMEOUT", "RESOURCE_NOT_READY"] {
        let mut measurer = FailingMeasurer(0, code);
        let error = compute_with_measurer(&fixture(390.0), &mut measurer).unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!(error.message, "synthetic provider failure");
        assert_eq!(
            measurer.0, 1,
            "stop querying after the first callback error"
        );
    }
}

#[test]
fn provider_error_from_final_width_scene_build_is_propagated() {
    let mut input = fixed_text();
    let mut second = input.nodes[1].clone();
    second.id = "fixed2".into();
    second.props.text = Some("another fixed text".into());
    input.nodes.push(second);
    for code in ["JOB_TIMEOUT", "RESOURCE_NOT_READY"] {
        let mut measurer = FailingMeasurer(0, code);
        let error = compute_with_measurer(&input, &mut measurer).unwrap_err();
        assert_eq!(error.code, code);
        assert_eq!(error.message, "synthetic provider failure");
        assert_eq!(measurer.0, 1);
    }
}

#[test]
fn validation_and_missing_font_fail_without_calling_provider() {
    let mut input = fixture(390.0);
    let mut measurer = RecordingMeasurer::default();
    input.viewport.width = f64::NAN;
    assert_eq!(
        compute_with_measurer(&input, &mut measurer)
            .unwrap_err()
            .code,
        "LAYOUT_INVALID"
    );
    assert!(measurer.0.is_empty());
    input.viewport.width = 390.0;
    input.fonts.clear();
    assert_eq!(
        compute_with_measurer(&input, &mut measurer)
            .unwrap_err()
            .code,
        "FONT_MISSING"
    );
    assert!(measurer.0.is_empty());
}

#[test]
fn trait_objects_are_supported_without_a_text_crate_dependency() {
    let mut provider = RecordingMeasurer::default();
    let measurer: &mut dyn TextMeasurer = &mut provider;
    assert!(compute_with_measurer(&fixed_text(), measurer).is_ok());
}
