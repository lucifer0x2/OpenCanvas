use super::*;
fn fixture() -> LayoutInput {
    serde_json::from_value(serde_json::json!({"schemaVersion":1,"projectId":"p","revision":1,"frameId":"f","viewport":{"width":300,"height":200},"nodes":[{"id":"f","parentId":null,"kind":"frame","props":{"layout":{"mode":"column","padding":10,"gap":5}}},{"id":"t","parentId":"f","kind":"text","props":{"width":"fill","height":"hug","text":"Tiếng Việt"}}],"fonts":[{"id":"font_bundled","family":"Noto Sans","hash":"h","url":"/font.ttf"}]})).unwrap()
}
#[test]
fn missing_measurements_are_batched_not_published() {
    let input = fixture();
    let LayoutStep::Pending { requests } = compute(&input, &MeasureCache::new()).unwrap() else {
        panic!("must be pending")
    };
    assert!(!requests.is_empty());
    assert!(requests.iter().all(|r| r.text == "Tiếng Việt"));
}
#[test]
fn missing_font_is_structured_failure() {
    let mut input = fixture();
    input.fonts.clear();
    assert_eq!(
        compute(&input, &MeasureCache::new()).unwrap_err().code,
        "FONT_MISSING"
    );
}
#[test]
fn rejects_cycles_and_non_finite_geometry() {
    let mut input = fixture();
    input.nodes[0].parent_id = Some("t".into());
    assert!(compute(&input, &MeasureCache::new()).is_err());
    input.nodes[0].parent_id = None;
    input.viewport.width = f32::NAN;
    assert!(compute(&input, &MeasureCache::new()).is_err());
}
#[test]
fn image_fit_scene_values_match_the_authored_vocabulary() {
    for fit in IMAGE_FITS {
        assert_eq!(scene_fit(Some(fit)), Some(fit));
    }
    assert_eq!(scene_fit(None), Some("contain"));
    assert_eq!(scene_fit(Some("fill")), Some("fit"), "legacy scene name");
    assert_eq!(scene_fit(Some("stretch")), None);
    let mut input = fixture();
    input.nodes.push(serde_json::from_value(serde_json::json!({"id":"i","parentId":"f","kind":"image","props":{"width":10,"height":10,"assetId":"a","fit":"stretch"}})).unwrap());
    let error = compute(&input, &MeasureCache::new()).unwrap_err();
    assert_eq!(error.message, "Unsupported image fit");
}
