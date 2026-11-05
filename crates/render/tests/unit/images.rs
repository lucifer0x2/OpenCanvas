use super::*;
use crate::{images, ResourceBytes};
use image::{
    codecs::{jpeg::JpegEncoder, png::PngEncoder, webp::WebPEncoder},
    ExtendedColorType, ImageEncoder,
};
use sha2::{Digest, Sha256};

fn image_scene(bytes: &[u8], media: &str) -> (PaintScene, String) {
    let hash = format!("{:x}", Sha256::digest(bytes));
    let mut n = node("im", 0.0, 0.0, 64.0, 64.0, "transparent");
    n.kind = "image".into();
    n.asset_id = Some("im".into());
    n.fit = "fit".into();
    let mut s = scene(vec![n]);
    s.assets.push(
        serde_json::from_value(json!({"id":"im","hash":hash,"url":"ignored","mediaType":media}))
            .unwrap(),
    );
    (s, hash)
}
#[test]
fn pure_png_jpeg_webp_decode_and_alpha() {
    for media in ["image/png", "image/jpeg", "image/webp"] {
        let mut bytes = Vec::new();
        match media {
            "image/png" => PngEncoder::new(&mut bytes)
                .write_image(&[255, 0, 0, 128], 1, 1, ExtendedColorType::Rgba8)
                .unwrap(),
            "image/jpeg" => JpegEncoder::new_with_quality(&mut bytes, 100)
                .encode(&[255, 0, 0], 1, 1, ExtendedColorType::Rgb8)
                .unwrap(),
            _ => WebPEncoder::new_lossless(&mut bytes)
                .write_image(&[255, 0, 0, 128], 1, 1, ExtendedColorType::Rgba8)
                .unwrap(),
        }
        let (s, hash) = image_scene(&bytes, media);
        let mut r = Resources::default();
        r.assets.insert(
            "im".into(),
            ResourceBytes {
                hash: &hash,
                bytes: &bytes,
            },
        );
        let r = crate::render(&s, &r, &RenderOptions::default()).unwrap();
        let p = pixel(&r, 32, 32);
        if media == "image/jpeg" {
            assert!(p[0] > 250 && p[1] < 5 && p[3] == 255);
        } else {
            assert_eq!(p, [128, 0, 0, 128]);
        }
    }
}
#[test]
fn image_fit_contain_cover_and_crop_use_bilinear_sampling() {
    let mut bytes = Vec::new();
    PngEncoder::new(&mut bytes)
        .write_image(
            &[255, 0, 0, 255, 0, 0, 255, 255],
            2,
            1,
            ExtendedColorType::Rgba8,
        )
        .unwrap();
    let (mut s, hash) = image_scene(&bytes, "image/png");
    let mut r = Resources::default();
    r.assets.insert(
        "im".into(),
        ResourceBytes {
            hash: &hash,
            bytes: &bytes,
        },
    );
    s.nodes[0].fit = "contain".into();
    let out = crate::render(&s, &r, &RenderOptions::default()).unwrap();
    assert_eq!(pixel(&out, 30, 1), [0; 4]);
    assert!(pixel(&out, 31, 30)[0] > 0 && pixel(&out, 31, 30)[2] > 0);
    s.nodes[0].fit = "cover".into();
    let out = crate::render(&s, &r, &RenderOptions::default()).unwrap();
    assert_eq!(pixel(&out, 30, 1)[3], 255);
    s.nodes[0].crop = Some(opencanvas_layout::Crop {
        x: 0.5,
        y: 0.0,
        width: 0.5,
        height: 1.0,
    });
    let out = crate::render(&s, &r, &RenderOptions::default()).unwrap();
    assert_eq!(pixel(&out, 50, 30), [0, 0, 255, 255]);
}
#[test]
fn icc_srgb_conversion_keeps_alpha_and_exif_orientation() {
    let mut bytes = Vec::new();
    let mut encoder = PngEncoder::new(&mut bytes);
    encoder
        .set_icc_profile(moxcms::ColorProfile::new_srgb().encode().unwrap())
        .unwrap();
    // Minimal little-endian TIFF, orientation 6 (90 clockwise), no external fixture.
    let exif = vec![
        b'I', b'I', 42, 0, 8, 0, 0, 0, 1, 0, 0x12, 1, 3, 0, 1, 0, 0, 0, 6, 0, 0, 0, 0, 0, 0, 0,
    ];
    encoder.set_exif_metadata(exif).unwrap();
    encoder
        .write_image(
            &[255, 0, 0, 128, 0, 0, 255, 255],
            2,
            1,
            ExtendedColorType::Rgba8,
        )
        .unwrap();
    let p = images::decode(&bytes, "image/png", &RenderOptions::default()).unwrap();
    assert_eq!((p.width(), p.height()), (1, 2));
    assert_eq!(p.pixels()[0].alpha(), 128);
    assert_eq!(p.pixels()[1].alpha(), 255);
}
#[test]
fn corrupt_image_readiness_and_png_straight_alpha_round_trip() {
    let (s, hash) = image_scene(b"bad", "image/png");
    let mut r = Resources::default();
    r.assets.insert(
        "im".into(),
        ResourceBytes {
            hash: &hash,
            bytes: b"bad",
        },
    );
    assert_eq!(
        crate::render(&s, &r, &RenderOptions::default())
            .unwrap_err()
            .code,
        "RESOURCE_NOT_READY"
    );
    let s = scene(vec![node("n", 0.0, 0.0, 64.0, 64.0, "#ff000080")]);
    let r = render(&s);
    let png = crate::encode_png(&r).unwrap();
    let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
    assert_eq!(decoded.get_pixel(30, 30).0, [255, 0, 0, 128]);
}
