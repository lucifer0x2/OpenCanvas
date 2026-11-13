//! `@set` rejects shorthands that would write part of an object prop (`update_node` replaces
//! object props whole); full objects and scalar edits compile as before.
use super::tests::ops;
use super::{compile, OcdError};
use serde_json::json;

const LAYOUT_HINT: &str = r#"layout={"mode":"column","padding":32,"gap":8}"#;
const OVERRIDES_HINT: &str = r#"overrides={"ph":{"text":"x","fill":{"tokenId":"pri"}}}"#;
const STROKE_HINT: &str = "stroke=#E5E7EB,1,inside or stroke={";
const PREFIX: &str = "@set card fill=#fff ";

/// Compiles `@set card fill=#fff <part>` on line 2 and returns the error it must raise.
fn rejected(part: &str) -> OcdError {
    let source = format!("@set a fill=#fff\n{PREFIX}{part}\n");
    let e = compile(&source).expect_err(&source);
    let column = PREFIX.chars().count() + 1;
    assert_eq!((e.line, e.column), (2, column), "{part}: {}", e.message);
    e
}

fn assert_says(e: &OcdError, texts: &[&str]) {
    for text in texts {
        assert!(e.message.contains(text), "{text}: {}", e.message);
    }
}

fn assert_replaced(e: &OcdError, object: &str, hint: &str) {
    let what = format!("would write part of `{object}`");
    assert_says(e, &[&what, "replaces the whole object", hint]);
}

#[test]
fn layout_keys_and_flags_are_rejected_with_the_full_layout_hint() {
    for part in [
        "gap=8",
        "gap=$g",
        "p=32",
        "p=0,12",
        "align=center",
        "justify=between",
        "row",
        "col",
        "free",
        "wrap",
        "padding=32",
    ] {
        assert_replaced(&rejected(part), "layout", LAYOUT_HINT);
    }
    // A dotted catalog key would be read as a descendant override; it names its object.
    assert_replaced(&rejected("layout.gap=8"), "layout", LAYOUT_HINT);
    assert_replaced(&rejected("stroke.width=2"), "stroke", STROKE_HINT);
}

#[test]
fn a_partial_stroke_is_rejected_and_a_full_one_compiles() {
    for part in ["stroke=#000", "stroke=$line", "stroke=#000,2"] {
        assert_replaced(&rejected(part), "stroke", STROKE_HINT);
    }
    assert_eq!(
        ops("@set a stroke=$line,2,center")[0]["props"],
        json!({"stroke":{"color":{"tokenId":"line"},"width":2,"align":"center"}})
    );
    assert_eq!(
        ops(r##"@set a stroke={"color":"#000"}"##)[0]["props"],
        json!({"stroke":{"color":"#000"}})
    );
}

#[test]
fn descendant_overrides_are_rejected_with_the_full_overrides_hint() {
    for part in ["ph.fill=$pri", "ph.text=Hi", "label.visible=false"] {
        assert_replaced(&rejected(part), "overrides", OVERRIDES_HINT);
    }
    for part in [r#"ph="x""#, r#"ph={"text":"x"}"#] {
        let e = rejected(part);
        assert!(e.message.starts_with("Unknown key `ph`"), "{}", e.message);
        assert_says(&e, &["replaces all overrides", OVERRIDES_HINT]);
    }
}

#[test]
fn full_objects_compile_to_update_node() {
    assert_eq!(
        ops(&format!("@set card {LAYOUT_HINT}")),
        vec![json!({"op":"update_node","id":"card",
            "props":{"layout":{"mode":"column","padding":32,"gap":8}}})]
    );
    assert_eq!(
        ops(&format!("@set email {OVERRIDES_HINT}")),
        vec![json!({"op":"update_node","id":"email",
            "props":{"overrides":{"ph":{"text":"x","fill":{"tokenId":"pri"}}}}})]
    );
    assert_eq!(
        ops(r#"@set card props={"layout":{"mode":"row"},"opacity":0.5}"#),
        vec![json!({"op":"update_node","id":"card",
            "props":{"layout":{"mode":"row"},"opacity":0.5}})]
    );
}

#[test]
fn scalar_edits_are_unchanged() {
    let source = "@set t text=\"Hi\" size=28/32 fill=$ink r=8 w=fill h=hug stroke=#E5E7EB,2,inside";
    let expected = json!({"op":"update_node","id":"t","props":{"text":"Hi","fontSize":28,
        "lineHeight":32,"fill":{"tokenId":"ink"},"radius":8,"width":"fill","height":"hug",
        "stroke":{"color":"#E5E7EB","width":2,"align":"inside"}}});
    assert_eq!(ops(source), vec![expected]);
    assert_eq!(
        ops("@set a .body 10x20 opacity=0.5")[0]["props"],
        json!({"styleId":"body","width":10,"height":20,"opacity":0.5})
    );
}

#[test]
fn node_creation_keeps_the_same_shorthands() {
    let source = "page#p\n  comp#input 10x10\n    text#ph \"x\"\n  \
        frame#card 100x100 col wrap p=32 gap=8 align=center justify=between\n    \
        use#email input ph=\"x\" ph.fill=$pri\n";
    let ops = ops(source);
    assert_eq!(
        ops[3]["props"]["layout"],
        json!({"mode":"column","wrap":true,"padding":32,"gap":8,"align":"center","justify":"space-between"})
    );
    assert_eq!(
        ops[4]["props"]["overrides"],
        json!({"ph":{"text":"x","fill":{"tokenId":"pri"}}})
    );
}
