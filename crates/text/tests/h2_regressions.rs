mod support;
use support::*;

#[test]
fn tracking_uses_logical_script_clusters_not_one_unit_per_mark() {
    let font = font();
    for (text, units) in [
        ("a\u{483}", 2.0),
        ("Ж\u{483}", 1.0),
        ("a\u{307}", 1.0),
        ("a\u{483}\u{307}", 2.0),
        ("a\u{306}\u{301}", 1.0),
    ] {
        let plain = font.positioned_run(text, 16.0, 0.0).unwrap();
        for spacing in [-2.0, 0.5] {
            let tracked = font.positioned_run(text, 16.0, spacing).unwrap();
            assert_eq!(
                tracked.advance,
                plain.advance + units * spacing as f64,
                "{text}"
            );
            assert!(tracked
                .glyphs
                .iter()
                .all(|g| text.is_char_boundary(g.cluster as usize)));
        }
    }
    let run = font.positioned_run("a\u{483}", 16.0, 0.5).unwrap();
    assert_eq!(
        run.glyphs.iter().map(|g| g.cluster).collect::<Vec<_>>(),
        [0, 1]
    );
}

#[test]
fn emergency_wrap_still_preserves_cross_script_extended_graphemes() {
    // Neither grapheme fits; shaping item boundaries must not become wrap breaks.
    assert_eq!(
        opencanvas_text::wrap_text("a\u{483}a\u{483}", |s| s.chars().count() <= 1),
        ["a\u{483}", "a\u{483}"]
    );
}

#[test]
fn negative_advance_expands_canvas_bounds_without_moving_mark_ink() {
    let font = font();
    let plain = font.positioned_run("\u{307}\u{301}", 16.0, 0.0).unwrap();
    let tracked = font.positioned_run("\u{307}\u{301}", 16.0, -2.0).unwrap();
    assert_eq!(tracked.advance, -2.0);
    for (a, b) in plain.glyphs.iter().zip(&tracked.glyphs) {
        assert_eq!((a.id, a.cluster, a.x, a.y), (b.id, b.cluster, b.x, b.y));
    }
    let outline = font.ink_bounds(&plain, 16.0).unwrap();
    let canvas = font.ink_bounds(&tracked, 16.0).unwrap();
    assert_eq!(canvas.x, -2.0);
    assert_eq!((canvas.y, canvas.height), (outline.y, outline.height));
    assert!(canvas.width > outline.width);
}

#[test]
fn captured_long_mixed_script_width_keeps_float_script_run_accumulation() {
    // Exact H1 Chromium values, bundled Noto bytes; no system-font fixture needed.
    let text = "αi\u{307}Жa\u{306}\u{301}ςj\u{307} ".repeat(24);
    let font = font();
    for (size, spacing, expected) in [
        (12.0, 0.0, 965.3756713867188),
        (16.0, -2.0, 951.1668701171875),
    ] {
        assert_eq!(
            font.positioned_run(&text, size, spacing).unwrap().advance,
            expected
        );
    }
}
