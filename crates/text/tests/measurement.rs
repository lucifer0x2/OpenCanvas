mod support;
use support::*;

#[test]
fn fixed_grid_positions_share_kerning_and_offsets_with_measurement() {
    let font = font();
    let fonts = set();
    for text in [
        "AVATAR",
        "To",
        "office ffi",
        "Tiếng Việt ặ ậ ệ ộ ợ ự",
        "a\u{306}\u{301}",
        "",
    ] {
        for spacing in [0.0, 1.25, -0.5] {
            let run = font.positioned_run(text, 16.0, spacing).unwrap();
            let mut request = request(text);
            request.letter_spacing = Some(spacing);
            let measurement = fonts.measure(&request).unwrap();
            assert_eq!(
                measurement.width,
                (run.advance.max(0.0) * 64.0).ceil() as f32 / 64.0
            );
            assert_eq!(measurement.lines[0].width, measurement.width);
            let mut pen = 0.0;
            for glyph in &run.glyphs {
                assert!(glyph.x.is_finite() && glyph.y.is_finite());
                assert!(text.is_char_boundary(glyph.cluster as usize));
                pen += glyph.x_advance;
            }
            assert_eq!(pen, run.advance);
        }
    }
    let av = font.positioned_run("AV", 40.0, 0.0).unwrap();
    let a = font.positioned_run("A", 40.0, 0.0).unwrap();
    let v = font.positioned_run("V", 40.0, 0.0).unwrap();
    assert!(av.advance < a.advance + v.advance, "kerning must stay on");
    let plain = font.positioned_run("abc", 16.0, 0.0).unwrap();
    let tracked = font.positioned_run("abc", 16.0, 1.25).unwrap();
    assert_eq!(
        tracked.advance - plain.advance,
        3.75,
        "includes trailing cluster tracking"
    );
    let lig = font.positioned_run("ffi", 16.0, 0.0).unwrap();
    let no_lig = font.positioned_run("ffi", 16.0, 1.0).unwrap();
    assert!(lig.glyphs.len() < no_lig.glyphs.len());
    let combining = font.positioned_run("a\u{306}\u{301}", 16.0, 1.0).unwrap();
    let untracked = font.positioned_run("a\u{306}\u{301}", 16.0, 0.0).unwrap();
    assert_eq!(combining.advance - untracked.advance, 1.0);
}

#[test]
fn baseline_height_fit_grid_and_empty_paragraphs() {
    let fonts = set();
    // Pin the intended macOS integer hhea metrics, independently of baseline arithmetic.
    for (size, ascent, descent) in [(12.0, 13.0, 4.0), (16.0, 17.0, 5.0), (24.0, 26.0, 7.0)] {
        let m = font().metrics(size).unwrap();
        assert_eq!((m.ascent, m.descent), (ascent, descent));
    }
    let mut r = request("hello\nworld\n");
    r.line_height = 23.25;
    let measured = fonts.measure(&r).unwrap();
    let metrics = font().metrics(r.font_size).unwrap();
    let baseline =
        metrics.ascent + ((r.line_height - metrics.ascent - metrics.descent) / 2.0).floor();
    assert_eq!(measured.ascent, metrics.ascent);
    assert_eq!(measured.height, 69.75);
    for (index, line) in measured.lines.iter().enumerate() {
        assert_eq!(line.y, index as f32 * r.line_height + baseline);
    }
    let mut r = request("hello world");
    let width = fonts.measure(&r).unwrap().width;
    r.width = Some(width);
    assert_eq!(fonts.measure(&r).unwrap().lines.len(), 1);
    r.width = Some(width - 1.0 / 64.0);
    assert_eq!(fonts.measure(&r).unwrap().lines.len(), 2);
    r.width = Some(-1.0);
    assert_eq!(fonts.measure(&r).unwrap().lines.len(), 10);
}

#[test]
fn missing_glyphs_are_sorted_bounded_and_use_notdef_without_fallback() {
    let font = font();
    let text = "⌘\u{10ffff}⌘\u{feff}\u{200d}\u{115f}\n\t";
    assert_eq!(
        font.missing_codepoints(text).unwrap(),
        vec![0x2318, 0x10ffff]
    );
    assert!(font
        .missing_codepoints("Tie\u{302}\u{301}ng Vie\u{302}\u{323}t")
        .unwrap()
        .is_empty());
    let run = font.positioned_run("⌘", 16.0, 0.0).unwrap();
    assert_eq!(run.glyphs[0].id.to_u32(), 0);
    assert!(run.advance > 0.0);
    let measurement = set().measure(&request("⌘")).unwrap();
    assert_eq!(measurement.missing, [0x2318]);
    assert!(!measurement.coverage_unavailable);
}

#[test]
fn outline_ink_bounds_use_same_run_and_ignore_whitespace() {
    let font = font();
    let run = font.positioned_run("Ag", 24.0, 0.0).unwrap();
    let ink = font.ink_bounds(&run, 24.0).unwrap();
    assert!(ink.y < 0.0 && ink.height > -ink.y && ink.width > 0.0);
    let spaces = font.positioned_run("  ", 24.0, 0.0).unwrap();
    let blank = font.ink_bounds(&spaces, 24.0).unwrap();
    assert_eq!((blank.width, blank.height), (0.0, 0.0));
    let tracked = font.positioned_run("Ag", 24.0, 2.0).unwrap();
    assert_eq!(
        font.ink_bounds(&tracked, 24.0).unwrap().width,
        ink.width + 2.0
    );
}
