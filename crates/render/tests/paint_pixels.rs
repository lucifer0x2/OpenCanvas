//! Public-API pixel regressions: quantize shaders once, preserve alpha, and
//! distinguish a hard image destination from its independently antialiased clip.
use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use opencanvas_render::{render, PaintScene, Raster, RenderOptions, ResourceBytes, Resources};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::cell::Cell;

fn scene(fill: Value, size: u32) -> PaintScene {
    serde_json::from_value(json!({
        "schemaVersion":1,"coordinateSpace":"frame-local",
        "provenance":{"projectId":"paint-test","revision":1,"frameId":"frame",
            "viewport":{"width":size,"height":size},"theme":"default","state":"default",
            "sceneVersion":"scene-3","rendererVersion":"web-4","sourceSceneKey":"paint",
            "fontManifestHash":"fonts","assetManifestHash":"assets"},
        "nodes":[{"id":"shape","parentId":null,"kind":"rectangle",
            "bounds":{"x":0,"y":0,"width":size,"height":size},"lines":[],
            "fontId":null,"fontSize":16,"lineHeight":24,"fill":fill,"assetId":null,
            "fit":"fit","opacity":1,"clip":false,"radius":0,"crop":null}],
        "fonts":[],"assets":[],"diagnostics":[]
    }))
    .unwrap()
}
fn pixel(r: &Raster, x: u32, y: u32) -> [u8; 4] {
    let p = ((y * r.width + x) * 4) as usize;
    r.rgba[p..p + 4].try_into().unwrap()
}
fn png(pixels: &[u8], w: u32, h: u32, orientation: bool) -> Vec<u8> {
    let mut bytes = Vec::new();
    let mut encoder = PngEncoder::new(&mut bytes);
    if orientation {
        // Self-authored EXIF TIFF: orientation 6, 90 degrees clockwise.
        encoder
            .set_exif_metadata(vec![
                b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 1, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0,
                0, 0,
            ])
            .unwrap();
    }
    encoder
        .write_image(pixels, w, h, ExtendedColorType::Rgba8)
        .unwrap();
    bytes
}
fn image(s: &mut PaintScene, bytes: &[u8]) -> Raster {
    let hash = format!("{:x}", Sha256::digest(bytes));
    s.nodes[0].kind = "image".into();
    s.nodes[0].asset_id = Some("image".into());
    s.assets = vec![serde_json::from_value(json!({
        "id":"image","hash":hash,"url":"unused","mediaType":"image/png"
    }))
    .unwrap()];
    let mut resources = Resources::default();
    resources
        .assets
        .insert("image".into(), ResourceBytes { hash: &hash, bytes });
    render(s, &resources, &RenderOptions::default()).unwrap()
}

#[test]
fn gradient_dithers_premultiplied_rgb_but_never_alpha() {
    let s = scene(
        json!({"type":"linear","angle":90,
        "stops":[[0,"#00ff0000"],[1,"#00ff0044"]]}),
        64,
    );
    let r = render(&s, &Resources::default(), &RenderOptions::default()).unwrap();
    // Pixel centers yield alpha 0.53125, 1.59375, 2.65625. The ordered
    // RGB offsets are -0.4921875, +0.2578125, -0.3046875 respectively.
    assert_eq!(pixel(&r, 0, 0), [0, 0, 0, 1]);
    assert_eq!(pixel(&r, 1, 0), [0, 2, 0, 2]);
    assert_eq!(pixel(&r, 2, 0), [0, 2, 0, 3]);
    for p in r.rgba.as_chunks::<4>().0 {
        assert!(p[..3].iter().all(|c| *c <= p[3]));
    }
}

#[test]
fn nonzero_alpha_gradient_interpolation_is_premultiplied_srgb() {
    let s = scene(
        json!({"type":"linear","angle":90,
        "stops":[[0,"#ff000040"],[1,"#0000ffc0"]]}),
        64,
    );
    let r = render(&s, &Resources::default(), &RenderOptions::default()).unwrap();
    let p = pixel(&r, 31, 20);
    assert!((31..=33).contains(&p[0]));
    assert!((94..=96).contains(&p[2]));
    assert_eq!(p[3], 127);
}

#[test]
fn image_filter_uses_four_bit_weights_and_single_final_truncation() {
    let bytes = png(&[255, 0, 0, 255, 0, 0, 255, 255], 2, 1, false);
    let r = image(&mut scene(json!("transparent"), 64), &bytes);
    assert_eq!(pixel(&r, 31, 20), [143, 0, 111, 255]);
    assert_eq!(pixel(&r, 32, 20), [127, 0, 127, 255]);
}

#[test]
fn exif_filter_quantization_remains_in_original_encoded_axes() {
    // A vertical red/blue original becomes horizontal blue/red. Reversing
    // the physical pixels alone would round the weight the wrong way.
    let bytes = png(&[255, 0, 0, 255, 0, 0, 255, 255], 1, 2, true);
    let r = image(&mut scene(json!("transparent"), 64), &bytes);
    assert_eq!(pixel(&r, 31, 20), [127, 0, 127, 255]);
    assert_eq!(pixel(&r, 32, 20), [143, 0, 111, 255]);
}

#[test]
fn contain_destination_is_hard_pixel_center_coverage_not_a_second_aa_clip() {
    let bytes = png(&[255, 0, 0, 255, 255, 0, 0, 255], 2, 1, false);
    let mut s = scene(json!("transparent"), 65);
    s.nodes[0].fit = "contain".into();
    let r = image(&mut s, &bytes);
    // The destination is [0,65] x [16.25,48.75], not integer aligned.
    for y in [16, 48] {
        assert_eq!(pixel(&r, 30, y), [255, 0, 0, 255]);
    }
    for y in [15, 49] {
        assert_eq!(pixel(&r, 30, y), [0; 4]);
    }
}

#[test]
fn image_filter_and_rounded_clip_preserve_premultiplied_alpha() {
    let bytes = png(&[255, 0, 0, 0, 0, 0, 255, 128], 2, 1, false);
    let mut s = scene(json!("transparent"), 64);
    s.nodes[0].radius = opencanvas_layout::Sides::All(15.0);
    let r = image(&mut s, &bytes);
    assert_eq!(pixel(&r, 0, 0), [0; 4]);
    assert!(r
        .rgba
        .as_chunks::<4>()
        .0
        .iter()
        .any(|p| p[3] > 0 && p[3] < 128));
    for p in r.rgba.as_chunks::<4>().0 {
        assert_eq!(p[0], 0); // Transparent red must not bleed into the blue.
        assert_eq!(p[2], p[3]);
    }
}

#[test]
fn tiled_shape_coverage_cancels_without_panicking_or_publishing_partial_pixels() {
    let s = scene(json!("#abcdef"), 1024);
    let calls = Cell::new(0);
    let cancelled = || {
        calls.set(calls.get() + 1);
        calls.get() >= 40
    };
    let options = RenderOptions {
        cancelled: Some(&cancelled),
        ..RenderOptions::default()
    };
    assert_eq!(
        render(&s, &Resources::default(), &options)
            .unwrap_err()
            .code,
        "CANCELLED"
    );
    assert_eq!(calls.get(), 40);
}
