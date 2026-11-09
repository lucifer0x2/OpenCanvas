use opencanvas_text::{transform_text, wrap_text};
use unicode_segmentation::UnicodeSegmentation;

#[test]
fn whitespace_matches_current_renderer_not_javascript_trim_or_spike() {
    let all_fit = |_: &str| true;
    assert_eq!(
        wrap_text(" \t aa\u{c}\u{b}bb \r\n\tcc\r\n\r dd \t", all_fit),
        ["aa bb", "cc", "", "dd"]
    );
    assert_eq!(wrap_text("", all_fit), [""]);
    assert_eq!(wrap_text("\n\n", all_fit), ["", "", ""]);
    assert_eq!(
        wrap_text("\u{2003}aa\u{2009}bb\u{2028}cc\u{2029}", all_fit),
        ["\u{2003}aa\u{2009}bb\u{2028}cc\u{2029}"]
    );
}

#[test]
fn nonbreaking_words_overflow_whole_not_emergency_broken() {
    for glue in ['\u{a0}', '\u{202f}', '\u{2060}', '\u{feff}'] {
        let word = format!("ab{glue}cd");
        assert_eq!(
            wrap_text(&format!("x {word} z"), |s| s.chars().count() <= 2),
            ["x", &word, "z"]
        );
        assert_eq!(wrap_text(&word, |_| false), [word]);
    }
}

#[test]
fn emergency_breaks_are_grapheme_safe_even_at_zero_width() {
    let word = "a\u{306}\u{301}👩\u{200d}💻🇻🇳b";
    let expected: Vec<_> = word.graphemes(true).map(str::to_owned).collect();
    assert_eq!(wrap_text(word, |_| false), expected);
    assert_eq!(wrap_text("abcde", |s| s.len() <= 2), ["ab", "cd", "e"]);
    assert_eq!(wrap_text("a b", |s| s.len() <= 3), ["a b"]);
}

#[test]
fn casing_has_expansions_context_and_preserves_authored_source() {
    let source = "Straße ﬁ a\u{306}\u{301}";
    assert_eq!(
        transform_text(source, Some("uppercase")).unwrap(),
        "STRASSE FI A\u{306}\u{301}"
    );
    assert_eq!(
        transform_text("ΟΣ ΟΣΑ ΟΣ\u{301} İ", Some("lowercase")).unwrap(),
        "ος οσα ος\u{301} i\u{307}"
    );
    assert_eq!(
        transform_text("Straße sao chép\nhello-world o'neil", Some("capitalize")).unwrap(),
        "Straße Sao Chép\nHello-World O'neil"
    );
    assert_eq!(
        transform_text("ßa ﬁle mIXED", Some("capitalize")).unwrap(),
        "ßa ﬁle MIXED"
    );
    assert_eq!(transform_text(source, None).unwrap(), source);
    assert_eq!(
        transform_text("123abc 45đẹp", Some("capitalize")).unwrap(),
        "123abc 45đẹp"
    );
}
