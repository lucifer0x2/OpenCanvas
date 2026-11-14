//! Parse errors name the right line and character column.
use super::{compile, OcdError};

fn error(source: &str) -> OcdError {
    compile(source).expect_err(source)
}

fn at(source: &str) -> (usize, usize) {
    let e = error(source);
    (e.line, e.column)
}

#[test]
fn unknown_key_reports_its_column_and_a_suggestion() {
    let e = error("page#p\n  frame#a 10x10 colr=#fff\n");
    assert_eq!((e.line, e.column), (2, 17));
    assert_eq!(e.message, "Unknown key `colr`");
    let e = error("page#p\n  frame#a 10x10 color=#fff\n");
    assert_eq!(e.suggestion.as_deref(), Some("fill"));
    let e = error("page#p\n  frame#a padding=4\n");
    assert_eq!(
        (e.line, e.column, e.suggestion.as_deref()),
        (2, 11, Some("p"))
    );
    let e = error("@set a radus=4");
    assert_eq!(
        (e.line, e.column, e.suggestion.as_deref()),
        (1, 8, Some("radius"))
    );
    // Columns count characters, not UTF-8 bytes.
    let e = error("page#p\n  text \"Đăng\" x=1\n");
    assert_eq!((e.line, e.column), (2, 8));
    assert_eq!(at("page#p\n  text name=\"Đăng\" bad=1\n"), (2, 20));
}

#[test]
fn bad_indent() {
    assert_eq!(at("page#p\n   frame#a\n"), (2, 4));
    assert_eq!(at("page#p\n    frame#a\n"), (2, 5));
    assert_eq!(at("page#p\n\tframe#a\n"), (2, 1));
    assert_eq!(at("frame#a\n"), (1, 1));
    assert_eq!(at("page#p\n  page#b\n"), (2, 3));
}

#[test]
fn unterminated_string_and_json() {
    let e = error("page#p\n  text#t \"Đăng nhập\n");
    assert_eq!((e.line, e.column), (2, 10));
    assert!(
        e.message.starts_with("Unterminated string"),
        "{}",
        e.message
    );
    assert_eq!(at("page#p\n  rect props={\"fill\":\"#000\"\n"), (2, 14));
    assert_eq!(at("page#p\n  rect props={\"fill\":}\n"), (2, 14));
}

#[test]
fn duplicate_ids_point_at_the_second_use() {
    let e = error("page#p\n  frame#a 1x1\n  frame#a 1x1\n");
    assert_eq!((e.line, e.column), (3, 9));
    assert_eq!(e.message, "Duplicate ID `a` (first used on line 2)");
    // An explicit ID may collide with an earlier generated one.
    let generated = super::compile("page#p\n  frame 1x1\n").unwrap().auto_ids[0]
        .1
        .clone();
    let e = error(&format!("page#p\n  frame 1x1\n  frame#{generated} 1x1\n"));
    assert_eq!((e.line, e.column), (3, 9));
    // A generated ID never takes one written earlier.
    let source = format!("page#p\n  frame#{generated} 1x1\n  frame 1x1\n");
    assert_ne!(super::compile(&source).unwrap().auto_ids[0].1, generated);
}

#[test]
fn other_syntax_errors() {
    assert_eq!(at("page#p\n  txt \"x\"\n"), (2, 3));
    assert_eq!(at("page#p\n  frame column\n"), (2, 9));
    assert_eq!(
        error("page#p\n  frame column\n").suggestion.as_deref(),
        Some("col")
    );
    assert_eq!(at("page#p\n  text \"a\" w=1\n"), (2, 8));
    assert_eq!(at("page#p\n  use#a\n"), (2, 3));
    assert_eq!(at("page#p\n  frame .a .b\n"), (2, 12));
    assert_eq!(at("@token x foo\n"), (1, 10));
    assert_eq!(at("@token x #fff type=colour\n"), (1, 20));
    assert_eq!(at("@style s w=1\n"), (1, 10));
    assert_eq!(at("@frob x\n"), (1, 1));
    assert_eq!(at("@in card\n  text \"x\"\n"), (2, 3));
    assert_eq!(at("@del a b=1\n"), (1, 8));
    assert_eq!(at("// only a comment\n\n"), (1, 1));
}

#[test]
fn more_than_500_operations_fail_at_the_crossing_line() {
    let source = "@del a\n".repeat(501);
    assert_eq!(at(&source), (501, 1));
    assert!(compile(&"@del a\n".repeat(500)).is_ok());
}
