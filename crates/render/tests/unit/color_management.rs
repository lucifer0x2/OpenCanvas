use crate::{images, RenderOptions};
use image::{codecs::png::PngEncoder, ExtendedColorType, ImageEncoder};
use moxcms::{
    ColorProfile, DataColorSpace, LutDataType, LutStore, LutType, LutWarehouse, Matrix3d,
};

#[test]
fn display_p3_profile_is_converted_to_srgb_not_just_tagged() {
    let mut bytes = Vec::new();
    let mut encoder = PngEncoder::new(&mut bytes);
    encoder
        .set_icc_profile(ColorProfile::new_display_p3().encode().unwrap())
        .unwrap();
    encoder
        .write_image(&[180, 100, 100, 128], 1, 1, ExtendedColorType::Rgba8)
        .unwrap();
    let p = images::decode(&bytes, "image/png", &RenderOptions::default()).unwrap();
    let pixel = p.pixels()[0];
    assert_eq!(pixel.alpha(), 128);
    // A wide-gamut muted red becomes a stronger sRGB red, preserving unassociated alpha.
    assert!(pixel.red() > 90);
    assert!(pixel.green() < 50);
}
#[test]
fn gray_icc_converts_luminance_and_preserves_source_alpha() {
    let mut bytes = Vec::new();
    let mut encoder = PngEncoder::new(&mut bytes);
    encoder
        .set_icc_profile(ColorProfile::new_gray_with_gamma(1.0).encode().unwrap())
        .unwrap();
    encoder
        .write_image(&[128, 128], 1, 1, ExtendedColorType::La8)
        .unwrap();
    let image = images::decode(&bytes, "image/png", &RenderOptions::default()).unwrap();
    let p = image.pixels()[0];
    assert_eq!(p.alpha(), 128);
    assert!(p.red() > 90);
    assert_eq!(p.red(), p.green());
    assert_eq!(p.green(), p.blue());
}

/// A single-MCU, 1x1, four-channel baseline JPEG. All DCT coefficients zero,
/// custom one-symbol DC/AC Huffman tables: every decoded ink equals 127.
fn cmyk_jpeg(profile: &[u8]) -> Vec<u8> {
    let mut out = vec![0xff, 0xd8];
    let segment = |out: &mut Vec<u8>, marker: u8, payload: &[u8]| {
        out.extend([0xff, marker]);
        out.extend(((payload.len() + 2) as u16).to_be_bytes());
        out.extend(payload);
    };
    let mut app = b"ICC_PROFILE\0\x01\x01".to_vec();
    app.extend(profile);
    segment(&mut out, 0xe2, &app);
    let mut quant = vec![0];
    quant.extend([1; 64]);
    segment(&mut out, 0xdb, &quant);
    let mut huffman = vec![0, 1];
    huffman.extend([0; 15]);
    huffman.push(0);
    huffman.extend([0x10, 1]);
    huffman.extend([0; 15]);
    huffman.push(0);
    segment(&mut out, 0xc4, &huffman);
    segment(
        &mut out,
        0xc0,
        &[
            8, 0, 1, 0, 1, 4, b'C', 0x11, 0, b'M', 0x11, 0, b'Y', 0x11, 0, b'K', 0x11, 0,
        ],
    );
    segment(
        &mut out,
        0xda,
        &[4, b'C', 0, b'M', 0, b'Y', 0, b'K', 0, 0, 63, 0],
    );
    out.extend([0, 0xff, 0xd9]);
    out
}
fn test_cmyk_profile() -> ColorProfile {
    let mut p = ColorProfile::new_srgb();
    p.color_space = DataColorSpace::Cmyk;
    p.cicp = None;
    // Synthetic 4D 2x2x2x2 constant-XYZ LUT. This is test data, not a production
    // printer profile. Constant PCS lets the test detect lost K/alpha confusion.
    p.lut_a_to_b_perceptual = Some(LutWarehouse::Lut(LutDataType {
        num_input_channels: 4,
        num_output_channels: 3,
        num_clut_grid_points: 2,
        matrix: Matrix3d::IDENTITY,
        num_input_table_entries: 256,
        num_output_table_entries: 256,
        input_table: LutStore::Store8((0..4).flat_map(|_| 0..=255).collect()),
        clut_table: LutStore::Store8(vec![70; 16 * 3]),
        output_table: LutStore::Store8((0..3).flat_map(|_| 0..=255).collect()),
        lut_type: LutType::Lut8,
    }));
    p
}
#[test]
fn cmyk_jpeg_icc_uses_original_inks_and_forces_opaque_alpha() {
    let profile = test_cmyk_profile().encode().unwrap();
    let parsed = ColorProfile::new_from_slice(&profile).unwrap();
    parsed
        .create_transform_8bit(
            moxcms::Layout::Rgba,
            &ColorProfile::new_srgb(),
            moxcms::Layout::Rgba,
            moxcms::TransformOptions::default(),
        )
        .unwrap();
    let bytes = cmyk_jpeg(&profile);
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(&bytes));
    assert_eq!(decoder.decode().unwrap(), [127; 4]);
    let image = images::decode(&bytes, "image/jpeg", &RenderOptions::default()).unwrap();
    assert_eq!((image.width(), image.height()), (1, 1));
    assert_eq!(image.pixels()[0].alpha(), 255);
    assert!(image.pixels()[0].red() > 100);
    // Generic untagged decoding is intentionally different from the constant-PCS profile.
    let untagged = cmyk_jpeg(&[]);
    let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(untagged));
    let inks = decoder.decode().unwrap();
    assert_eq!(inks, [127; 4]);
}
