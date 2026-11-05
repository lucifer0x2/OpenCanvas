use super::*;

#[test]
fn public_f64_scales_preserve_ceil_without_f32_roundtrip() {
    for (width, height, scale, expected) in [
        (40.0, 40.0, 2.4, (96, 96)),
        (320.0, 200.0, 0.3, (96, 60)),
        (133.5, 28.0, 3.0, (401, 84)),
        (40.0, 80.0, 96.0 / 80.0, (48, 96)),
        (40.00001, 40.0, 2.4, (97, 96)),
    ] {
        let mut scene = scene(vec![]);
        scene.provenance.viewport.width = width;
        scene.provenance.viewport.height = height;
        let raster = crate::render(
            &scene,
            &Resources::default(),
            &RenderOptions {
                scale,
                ..Default::default()
            },
        )
        .unwrap();
        assert_eq!((raster.width, raster.height), expected);
    }
}

#[test]
fn resource_inspection_has_no_png_target_budget_and_retains_strict_failure() {
    let mut scene = scene(vec![]);
    scene.provenance.viewport.width = 1e9;
    scene.provenance.viewport.height = 1e9;
    assert!(
        crate::inspect_resources(&scene, &Resources::default(), &RenderOptions::default())
            .unwrap()
            .is_empty()
    );
    scene.assets.push(serde_json::from_value(json!({"id":"missing","hash":"0".repeat(64),"mediaType":"image/png","url":"never-fetch"})).unwrap());
    let failure =
        crate::inspect_resources(&scene, &Resources::default(), &RenderOptions::default())
            .unwrap_err();
    assert_eq!(failure.code, "RESOURCE_NOT_READY");
    assert_eq!(failure.readiness[0].id, "missing");
    assert_eq!(failure.readiness[0].status, "failed");
}
