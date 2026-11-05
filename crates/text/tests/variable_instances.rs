#[path = "support/outlines.rs"]
mod outlines;
mod support;
use opencanvas_text::Font;
use support::*;

#[test]
fn manifest_weight_selects_real_outlines_and_advances_without_optical_axis() {
    let mut previous = None;
    for weight in [100, 400, 500, 700, 900] {
        let font = font().with_face("private alias", weight, "normal").unwrap();
        assert_eq!(font.hash(), HASH);
        assert_eq!(font.sfnt_bytes(), BYTES);
        let instance = font.shaping_font_at_size(32.0).unwrap();
        assert_eq!(
            instance.normalized_coords(),
            font.shaping_font().normalized_coords()
        );
        let run = font.positioned_run("AVATAR", 32.0, 0.0).unwrap();
        let paths = outlines::outlines(&font, 32.0, "AVATAR");
        if let Some((advance, old_paths)) = previous {
            assert!(run.advance > advance);
            assert_ne!(paths, old_paths, "outlines, not just widths, must vary");
        }
        previous = Some((run.advance, paths));
    }
}

#[test]
fn rebinding_manifest_weight_returns_to_default_without_stale_coordinates() {
    let default = font();
    let rebound = font()
        .with_face("alias", 900, "normal")
        .unwrap()
        .with_face("new alias", 400, "italic")
        .unwrap();
    assert!(rebound.shaping_font().normalized_coords().is_empty());
    assert_eq!(
        outlines::outlines(&rebound, 24.0, "Ag"),
        outlines::outlines(&default, 24.0, "Ag")
    );
    assert_eq!(
        rebound.positioned_run("AVATAR", 24.0, 0.0).unwrap().advance,
        default.positioned_run("AVATAR", 24.0, 0.0).unwrap().advance
    );
}

#[test]
fn static_regular_bytes_with_bold_italic_descriptors_do_not_get_faux_outlines() {
    let bytes = include_bytes!("../../../fixtures/fonts/Lato-Regular.ttf");
    let regular = Font::from_bytes(bytes, &hash(bytes)).unwrap();
    let alias = regular.clone().with_face("F2", 700, "italic").unwrap();
    assert!(alias.shaping_font().axes().is_empty());
    for size in [16.0, 24.0, 32.0] {
        assert_eq!(
            outlines::outlines(&alias, size, "Ag"),
            outlines::outlines(&regular, size, "Ag")
        );
        assert_eq!(
            alias
                .positioned_run("AVATAR office", size, 0.0)
                .unwrap()
                .advance,
            regular
                .positioned_run("AVATAR office", size, 0.0)
                .unwrap()
                .advance
        );
    }
}

#[test]
fn metrics_use_selected_instance_including_mvar_hhea_deltas() {
    // Controlled OFL table mutation: rename Noto's existing stro variation to
    // hasc, preserving its real variation store and sorted MVAR records. The
    // original fixtures do not vary hhea; this checks the instance plumbing.
    let mut bytes = BYTES.to_vec();
    let record = BYTES[12..]
        .as_chunks::<16>()
        .0
        .iter()
        .take(u16::from_be_bytes([BYTES[4], BYTES[5]]) as usize)
        .find(|r| &r[..4] == b"MVAR")
        .unwrap();
    let mvar = u32::from_be_bytes(record[8..12].try_into().unwrap()) as usize;
    assert_eq!(&bytes[mvar + 12..mvar + 16], b"stro");
    bytes[mvar + 12..mvar + 16].copy_from_slice(b"hasc");
    let font = Font::from_bytes(&bytes, &hash(&bytes)).unwrap();
    let mut ascents = Vec::new();
    for weight in [100, 400, 900] {
        let font = font
            .clone()
            .with_face("MVAR plumbing", weight, "normal")
            .unwrap();
        let instance = font.shaping_font_at_size(32.0).unwrap();
        let raw = instance.metrics().hhea_line.unwrap();
        let expected = (raw.ascender.to_f64() * 32.0 / 1000.0).round() as f32;
        assert_eq!(font.metrics(32.0).unwrap().ascent, expected);
        ascents.push(expected);
    }
    assert_ne!(
        ascents[0], ascents[2],
        "test must exercise actual MVAR delta"
    );
}
