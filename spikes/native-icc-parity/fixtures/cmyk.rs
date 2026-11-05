//! MIT repository-authored ICC/JPEG test math, copied from render's CMS unit fixture.
use moxcms::{
    ColorProfile, DataColorSpace, LutDataType, LutStore, LutType, LutWarehouse, Matrix3d,
    ProfileClass,
};

pub fn profile() -> ColorProfile {
    let mut p = ColorProfile::new_srgb();
    p.color_space = DataColorSpace::Cmyk;
    p.profile_class = ProfileClass::InputDevice;
    p.cicp = None;
    p.red_trc = None;
    p.green_trc = None;
    p.blue_trc = None;
    p.chromatic_adaptation = None;
    p.media_white_point = Some(p.white_point);
    p.red_colorant = Default::default();
    p.green_colorant = Default::default();
    p.blue_colorant = Default::default();
    // A bounded 4D constant-PCS test profile, NOT a printer characterization.
    p.lut_a_to_b_perceptual = Some(LutWarehouse::Lut(LutDataType {
        num_input_channels: 4,
        num_output_channels: 3,
        num_clut_grid_points: 2,
        matrix: Matrix3d::IDENTITY,
        num_input_table_entries: 256,
        num_output_table_entries: 256,
        // Same normalized math as the original unit fixture, but canonical lut16.
        // moxcms 0.8.1's lut8 writer wrongly emits lut16 table-count fields.
        input_table: LutStore::Store16((0..4).flat_map(|_| (0..=255).map(|v| v * 257)).collect()),
        clut_table: LutStore::Store16(vec![70 * 257; 16 * 3]),
        output_table: LutStore::Store16((0..3).flat_map(|_| (0..=255).map(|v| v * 257)).collect()),
        lut_type: LutType::Lut16,
    }));
    p
}

fn segment(out: &mut Vec<u8>, marker: u8, payload: &[u8]) {
    out.extend([0xff, marker]);
    out.extend(((payload.len() + 2) as u16).to_be_bytes());
    out.extend(payload);
}

pub fn jpeg(profile: Option<&[u8]>) -> Vec<u8> {
    let mut out = vec![0xff, 0xd8];
    if let Some(profile) = profile {
        let mut app = b"ICC_PROFILE\0\x01\x01".to_vec();
        app.extend(profile);
        segment(&mut out, 0xe2, &app);
    }
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
    // Single MCU, four components, no Adobe APP14 marker. All DCT coefficients 0.
    segment(
        &mut out,
        0xc0,
        &[
            8, 0, 8, 0, 8, 4, b'C', 0x11, 0, b'M', 0x11, 0, b'Y', 0x11, 0, b'K', 0x11, 0,
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn original_inks_and_icc_are_valid_not_rgba() {
        let p = super::super::profile::encode(&profile());
        let parsed = ColorProfile::new_from_slice(&p).unwrap();
        assert_eq!(parsed.color_space, DataColorSpace::Cmyk);
        let Some(LutWarehouse::Lut(lut)) = &parsed.lut_a_to_b_perceptual else {
            panic!("constant PCS LUT missing");
        };
        assert_eq!(lut.clut_table, LutStore::Store16(vec![70 * 257; 16 * 3]));
        assert_eq!(
            lut.input_table,
            LutStore::Store16((0..4).flat_map(|_| (0..=255).map(|v| v * 257)).collect())
        );
        parsed
            .create_transform_8bit(
                moxcms::Layout::Rgba,
                &ColorProfile::new_srgb(),
                moxcms::Layout::Rgba,
                moxcms::TransformOptions::default(),
            )
            .unwrap();
        let mut decoder = jpeg_decoder::Decoder::new(std::io::Cursor::new(jpeg(Some(&p))));
        assert_eq!(decoder.decode().unwrap(), vec![127; 8 * 8 * 4]);
        assert_eq!(decoder.icc_profile().as_deref(), Some(p.as_slice()));
    }
}
