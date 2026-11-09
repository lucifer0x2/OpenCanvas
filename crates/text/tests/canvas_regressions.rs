mod support;
use support::*;

#[test]
fn canvas_separators_shape_as_space_but_keep_utf8_clusters_and_source_lines() {
    let font = font();
    let plain = font.positioned_run("a b c", 16.0, 0.0).unwrap();
    for text in ["a\u{2028}b\u{2029}c", "a\nb\rc", "a\tb\u{c}c"] {
        let run = font.positioned_run(text, 16.0, 0.0).unwrap();
        assert_eq!(run.advance, plain.advance);
        assert!(run
            .glyphs
            .iter()
            .all(|g| text.is_char_boundary(g.cluster as usize)));
    }
    let measured = set().measure(&request("a\u{2028}b\u{2029}c")).unwrap();
    assert_eq!(measured.lines.len(), 1);
    assert_eq!(measured.lines[0].text, "a\u{2028}b\u{2029}c");
}

#[test]
fn default_ignorables_do_not_receive_independent_tracking() {
    let font = font();
    let text = "A\u{2060}B\u{200b}C";
    let normal = font.positioned_run(text, 16.0, 0.0).unwrap();
    for spacing in [-2.0, 0.5] {
        let tracked = font.positioned_run(text, 16.0, spacing).unwrap();
        assert_eq!(tracked.advance - normal.advance, 3.0 * spacing as f64);
        let ignorable = font
            .positioned_run("\u{2060}\u{200b}\u{feff}", 16.0, spacing)
            .unwrap();
        assert_eq!(ignorable.advance, 0.0);
    }
}

#[test]
fn zero_line_height_uses_font_bbox_baseline_without_rejecting_or_dividing() {
    let mut r = request("Ắgj\nType");
    r.font_size = 17.5;
    r.line_height = 0.0;
    let m = set().measure(&r).unwrap();
    // Pinned Chromium 156.0.8078.4 empty-string font bbox, Noto bytes in support.
    assert_eq!(m.ascent, 19.0);
    assert_eq!(m.height, 0.0);
    assert_eq!(m.lines.iter().map(|l| l.y).collect::<Vec<_>>(), [7.0, 7.0]);
    assert_eq!(
        m.lines.iter().map(|l| l.width).collect::<Vec<_>>(),
        [26.46875, 38.9375]
    );
}

#[test]
fn no_optical_axis_keeps_default_instance_and_public_api() {
    let font = font();
    for size in [12.0, 16.0, 17.5, 32.0] {
        let instance = font.shaping_font_at_size(size).unwrap();
        assert_eq!(
            instance.normalized_coords(),
            font.shaping_font().normalized_coords()
        );
    }
    assert!(font.shaping_font_at_size(f32::NAN).is_err());
}

#[test]
fn optical_instance_normalizes_once_and_clamps_without_font_specific_rules() {
    // Derive a synthetic optical axis from the existing OFL test font. No binary
    // fixture/download/system-font dependency. Only fvar metadata/avar mapping is
    // changed; shaping data remains intact, and the raw upload hash is recomputed.
    let mut bytes = BYTES.to_vec();
    let table = |tag: &[u8]| {
        BYTES[12..12 + u16::from_be_bytes([BYTES[4], BYTES[5]]) as usize * 16]
            .as_chunks::<16>()
            .0
            .iter()
            .find(|r| &r[..4] == tag)
            .map(|r| u32::from_be_bytes(r[8..12].try_into().unwrap()) as usize)
            .unwrap()
    };
    let fvar = table(b"fvar");
    let axis = fvar + u16::from_be_bytes([bytes[fvar + 4], bytes[fvar + 5]]) as usize;
    bytes[axis..axis + 4].copy_from_slice(b"opsz");
    for (offset, value) in [(4, 14_i32), (8, 14), (12, 32)] {
        bytes[axis + offset..axis + offset + 4].copy_from_slice(&(value * 65536).to_be_bytes());
    }
    let avar = table(b"avar");
    let count = u16::from_be_bytes([bytes[avar + 8], bytes[avar + 9]]) as usize;
    for i in 0..count {
        let at = avar + 10 + i * 4;
        let from: [u8; 2] = bytes[at..at + 2].try_into().unwrap();
        bytes[at + 2..at + 4].copy_from_slice(&from);
    }
    let font = opencanvas_text::Font::from_bytes(&bytes, &hash(&bytes)).unwrap();
    let instance = font.shaping_font_at_size(16.0).unwrap();
    assert_eq!(instance.normalized_coords()[0].to_bits(), 1820);
    assert!(font
        .shaping_font_at_size(12.0)
        .unwrap()
        .normalized_coords()
        .is_empty());
    assert_eq!(
        font.shaping_font_at_size(100.0)
            .unwrap()
            .normalized_coords()[0]
            .to_bits(),
        16384
    );
}
