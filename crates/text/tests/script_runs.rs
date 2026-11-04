mod support;
use support::*;

#[test]
fn mixed_script_items_keep_global_clusters_positions_and_tracking() {
    let font = font();
    for spacing in [0.0, -2.0, 0.5] {
        // Include leading/trailing neutrals and marks before/after script boundaries.
        let parts = [
            " . α\u{301} ",
            "i\u{307}j\u{307} ",
            "Ж\u{301} ",
            "a\u{306}\u{301}",
        ];
        let text = parts.concat();
        let whole = font.positioned_run(&text, 16.0, spacing).unwrap();
        let mut pen = 0.0;
        let mut byte = 0;
        let mut at = 0;
        for part in parts {
            let local = font.positioned_run(part, 16.0, spacing).unwrap();
            for glyph in local.glyphs {
                let actual = &whole.glyphs[at];
                assert_eq!(actual.id, glyph.id);
                assert_eq!(actual.cluster, glyph.cluster + byte);
                assert_eq!(actual.x, glyph.x + pen);
                assert_eq!(actual.y, glyph.y);
                assert_eq!(actual.x_advance, glyph.x_advance);
                assert_eq!(actual.y_advance, glyph.y_advance);
                assert!(text.is_char_boundary(actual.cluster as usize));
                at += 1;
            }
            pen += local.advance;
            byte += part.len() as u32;
        }
        assert_eq!(whole.glyphs.len(), at);
        assert_eq!(whole.advance, pen);
    }
}

#[test]
fn combining_marks_stay_with_base_across_many_script_changes() {
    let font = font();
    let text = "αi\u{307}Жa\u{306}\u{301}ςj\u{307} ".repeat(128);
    let run = font.positioned_run(&text, 17.5, 0.5).unwrap();
    // Canvas sums exact fixed-point glyph advances within each script item, then
    // accumulates item widths as float. A long run need not equal the f64 glyph sum.
    let parts = ["α", "i\u{307}", "Ж", "a\u{306}\u{301}", "ς", "j\u{307} "];
    let mut expected = 0.0_f32;
    let mut at = 0;
    for part in parts.into_iter().cycle().take(128 * parts.len()) {
        let local = font.positioned_run(part, 17.5, 0.5).unwrap();
        for glyph in &local.glyphs {
            assert_eq!(run.glyphs[at].x, expected as f64 + glyph.x);
            assert_eq!(run.glyphs[at].x_advance, glyph.x_advance);
            at += 1;
        }
        expected += local.advance as f32;
    }
    assert_eq!(run.advance, expected as f64);
    assert!(run.glyphs.windows(2).all(|w| w[0].cluster <= w[1].cluster));
    for glyph in &run.glyphs {
        assert!(text.is_char_boundary(glyph.cluster as usize));
        // Marks may merge with a base cluster, never with the preceding script item.
        assert!(!matches!(
            text[glyph.cluster as usize..].chars().next(),
            Some('\u{307}' | '\u{306}' | '\u{301}')
        ));
    }
}
