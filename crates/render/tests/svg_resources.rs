//! SVG validates original-resource integrity without asking a raster painter to paint it.
use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use opencanvas_render::{
    inspect_svg_resources, PaintScene, RenderOptions, ResourceBytes, Resources,
};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn scene(fonts: Value, assets: Value) -> PaintScene {
    serde_json::from_value(json!({"schemaVersion":1,"coordinateSpace":"frame-local",
        "provenance":{"projectId":"p","revision":1,"frameId":"f",
            "viewport":{"width":10,"height":10},"theme":"default","state":"default",
            "sceneVersion":"scene-3","rendererVersion":"web-4","sourceSceneKey":"opaque",
            "fontManifestHash":"fonts","assetManifestHash":"assets"},
        "nodes":[],"fonts":fonts,"assets":assets,"diagnostics":[]}))
    .unwrap()
}

fn font_scene(hash: &str) -> PaintScene {
    scene(
        json!([{"id":"font","family":"Probe","weight":400,"style":"normal",
        "hash":hash,"url":"never-fetch"}]),
        json!([]),
    )
}

fn image_scene(hash: &str) -> PaintScene {
    scene(
        json!([]),
        json!([{"id":"image","mediaType":"image/png",
        "hash":hash,"url":"never-fetch"}]),
    )
}

#[test]
fn svg_font_structure_face_missing_and_hash_validation_remain_strict() {
    let mut bytes = include_bytes!("../../../renderer/fonts/NotoSans.ttf").to_vec();
    // A structural control only, NOT a valid COLRv1 fixture (captured test in server).
    let count = u16::from_be_bytes([bytes[4], bytes[5]]) as usize;
    let entry = bytes[12..12 + count * 16]
        .as_chunks_mut::<16>()
        .0
        .iter_mut()
        .find(|e| &e[..4] == b"name")
        .unwrap();
    entry[..4].copy_from_slice(b"COLR");
    let digest = hash(&bytes);
    let mut resources = Resources::default();
    resources.fonts.insert(
        "font".into(),
        ResourceBytes {
            hash: &digest,
            bytes: &bytes,
        },
    );
    let scene = font_scene(&digest);
    let ready = inspect_svg_resources(&scene, &resources, &RenderOptions::default()).unwrap();
    assert_eq!(ready[0].status, "ready");
    let mut invalid_face = scene.clone();
    invalid_face.fonts[0].style = "invalid".into();
    assert_eq!(
        inspect_svg_resources(&invalid_face, &resources, &RenderOptions::default())
            .unwrap_err()
            .code,
        "RESOURCE_NOT_READY"
    );
    let changed = font_scene(&"0".repeat(64));
    assert_eq!(
        inspect_svg_resources(&changed, &resources, &RenderOptions::default())
            .unwrap_err()
            .code,
        "PIPELINE_MISMATCH"
    );
    let error = inspect_svg_resources(&scene, &Resources::default(), &RenderOptions::default())
        .unwrap_err();
    assert_eq!(error.code, "RESOURCE_NOT_READY");
    assert_eq!(error.readiness[0].status, "failed");
    let corrupt = b"not a font";
    let digest = hash(corrupt);
    resources.fonts.insert(
        "font".into(),
        ResourceBytes {
            hash: &digest,
            bytes: corrupt,
        },
    );
    let error = inspect_svg_resources(&font_scene(&digest), &resources, &RenderOptions::default())
        .unwrap_err();
    assert_eq!(error.code, "RESOURCE_NOT_READY");
    assert_eq!(error.readiness[0].code.as_deref(), Some("FONT_MISSING"));
}

#[test]
fn svg_original_codec_decode_rejects_truncation_and_changed_bytes_without_color_conversion() {
    let mut bytes = Vec::new();
    let mut encoder = PngEncoder::new(&mut bytes);
    encoder
        .set_icc_profile(moxcms::ColorProfile::new_display_p3().encode().unwrap())
        .unwrap();
    encoder
        .write_image(&[180, 100, 100, 128], 1, 1, ExtendedColorType::Rgba8)
        .unwrap();
    let original = bytes.clone();
    let digest = hash(&bytes);
    let mut resources = Resources::default();
    resources.assets.insert(
        "image".into(),
        ResourceBytes {
            hash: &digest,
            bytes: &bytes,
        },
    );
    let ready = inspect_svg_resources(&image_scene(&digest), &resources, &RenderOptions::default())
        .unwrap();
    assert_eq!(ready[0].status, "ready");
    assert_eq!(bytes, original);
    resources.assets.insert(
        "image".into(),
        ResourceBytes {
            hash: &digest,
            bytes: &bytes[..33],
        },
    );
    assert_eq!(
        inspect_svg_resources(&image_scene(&digest), &resources, &RenderOptions::default())
            .unwrap_err()
            .code,
        "PIPELINE_MISMATCH"
    );
    let truncated_hash = hash(&bytes[..33]);
    resources.assets.insert(
        "image".into(),
        ResourceBytes {
            hash: &truncated_hash,
            bytes: &bytes[..33],
        },
    );
    let error = inspect_svg_resources(
        &image_scene(&truncated_hash),
        &resources,
        &RenderOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, "RESOURCE_NOT_READY");
    assert_eq!(
        error.readiness[0].code.as_deref(),
        Some("ASSET_DECODE_FAILED")
    );
    let error = inspect_svg_resources(
        &image_scene(&digest),
        &Resources::default(),
        &RenderOptions::default(),
    )
    .unwrap_err();
    assert_eq!(error.code, "RESOURCE_NOT_READY");
    assert_eq!(error.readiness[0].status, "failed");
}

#[test]
fn svg_honors_cancellation_pipeline_identity_and_upstream_readiness() {
    let scene = scene(json!([]), json!([]));
    let options = RenderOptions {
        cancelled: Some(&|| true),
        ..Default::default()
    };
    assert_eq!(
        inspect_svg_resources(&scene, &Resources::default(), &options)
            .unwrap_err()
            .code,
        "CANCELLED"
    );
    let mut old = scene.clone();
    old.provenance.scene_version = "old".into();
    assert_eq!(
        inspect_svg_resources(&old, &Resources::default(), &RenderOptions::default())
            .unwrap_err()
            .code,
        "PIPELINE_MISMATCH"
    );
    let resources = Resources {
        readiness_diagnostics: vec![json!({"code":"RESOURCE_UNAVAILABLE"})],
        ..Default::default()
    };
    assert_eq!(
        inspect_svg_resources(&scene, &resources, &RenderOptions::default())
            .unwrap_err()
            .code,
        "RESOURCE_NOT_READY"
    );
}
