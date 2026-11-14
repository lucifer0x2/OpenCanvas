//! `@set`: whole values replace as before; partial writes of an object prop (layout keys and
//! flags, `<prop>.<field>=`, descendant overrides, a partial `stroke`) compile to a merging
//! `update_node`, after a replacing one when the statement also writes object values.
use super::tests::ops;
use super::{compile, OcdError};
use serde_json::{json, Value};

const LAYOUT: &str = r#"layout={"mode":"column","padding":32,"gap":8}"#;
const OVERRIDES: &str = r#"overrides={"ph":{"text":"x","fill":{"tokenId":"pri"}}}"#;

fn merged(props: Value) -> Vec<Value> {
    vec![json!({"op":"update_node","id":"card","props":props,"merge":true})]
}

fn replaced(props: Value) -> Value {
    json!({"op":"update_node","id":"card","props":props})
}

fn error(source: &str) -> OcdError {
    compile(source).expect_err(source)
}

#[test]
fn layout_keys_flags_and_fields_merge_into_the_stored_layout() {
    let cases = [
        ("gap=8", json!({"gap":8})),
        ("gap=$g", json!({"gap":{"tokenId":"g"}})),
        ("p=0,12", json!({"padding":[0,12]})),
        ("justify=between", json!({"justify":"space-between"})),
        (
            "col wrap align=center",
            json!({"mode":"column","wrap":true,"align":"center"}),
        ),
        ("layout.gap=8", json!({"gap":8})),
        (
            "layout.padding=8,16 free",
            json!({"padding":[8,16],"mode":"free"}),
        ),
    ];
    for (part, layout) in cases {
        assert_eq!(
            ops(&format!("@set card {part}")),
            merged(json!({"layout":layout})),
            "{part}"
        );
    }
    assert_eq!(
        ops("@set card stroke.width=2 stroke.color=$line"),
        merged(json!({"stroke":{"width":2,"color":{"tokenId":"line"}}}))
    );
}

#[test]
fn a_partial_stroke_merges_and_a_full_one_replaces() {
    assert_eq!(
        ops("@set card stroke=#000"),
        merged(json!({"stroke":{"color":"#000"}}))
    );
    assert_eq!(
        ops("@set card stroke=$line,2"),
        merged(json!({"stroke":{"color":{"tokenId":"line"},"width":2}}))
    );
    assert_eq!(
        ops("@set card stroke=$line,2,center"),
        vec![replaced(
            json!({"stroke":{"color":{"tokenId":"line"},"width":2,"align":"center"}})
        )]
    );
    assert_eq!(
        ops(r##"@set card stroke={"color":"#000"}"##),
        vec![replaced(json!({"stroke":{"color":"#000"}}))]
    );
}

#[test]
fn descendant_keys_merge_one_override_and_bare_ones_fail() {
    assert_eq!(
        ops("@set card ph.fill=$pri ph.text=Hi label.visible=false"),
        merged(
            json!({"overrides":{"ph":{"fill":{"tokenId":"pri"},"text":"Hi"},
            "label":{"visible":false}}})
        )
    );
    for part in [r#"ph="x""#, r#"ph={"text":"x"}"#, "opacty=4"] {
        let e = error(&format!("@set card {part}"));
        assert_eq!((e.line, e.column), (1, 11), "{part}");
        assert!(e.message.starts_with("Unknown key"), "{}", e.message);
        assert!(e.message.contains("<descendantId>.text="), "{}", e.message);
    }
    assert_eq!(
        error("@set card opacty=4").suggestion.as_deref(),
        Some("opacity")
    );
    let e = error("@set card layout.=4");
    assert_eq!((e.line, e.column), (1, 18), "{}", e.message);
}

#[test]
fn mixed_statements_replace_objects_then_merge_the_rest() {
    // Scalars and references replace in a merge patch too: one merging update.
    assert_eq!(
        ops("@set card fill=$ink r=8 gap=8"),
        merged(json!({"fill":{"tokenId":"ink"},"radius":8,"layout":{"gap":8}}))
    );
    // A whole object replaces first; the partial writes merge after it.
    assert_eq!(
        ops("@set card stroke=#000,1,inside opacity=0.5 gap=8"),
        vec![
            replaced(json!({"stroke":{"color":"#000","width":1,"align":"inside"}})),
            json!({"op":"update_node","id":"card","merge":true,
                "props":{"opacity":0.5,"layout":{"gap":8}}}),
        ]
    );
    // Partial writes of a prop also written whole complete that object.
    assert_eq!(
        ops(r#"@set card gap=8 layout={"mode":"row"}"#),
        vec![replaced(json!({"layout":{"mode":"row","gap":8}}))]
    );
    let compiled = compile("@set card stroke=#000,1,inside gap=8").unwrap();
    assert_eq!(compiled.positions, [(1, 1), (1, 1)]);
}

#[test]
fn full_objects_and_scalars_compile_to_one_replacing_update() {
    assert_eq!(
        ops(&format!("@set card {LAYOUT}")),
        vec![replaced(
            json!({"layout":{"mode":"column","padding":32,"gap":8}})
        )]
    );
    assert_eq!(
        ops(&format!("@set card {OVERRIDES}")),
        vec![replaced(
            json!({"overrides":{"ph":{"text":"x","fill":{"tokenId":"pri"}}}})
        )]
    );
    assert_eq!(
        ops(r#"@set card props={"layout":{"mode":"row"},"opacity":0.5}"#),
        vec![replaced(json!({"layout":{"mode":"row"},"opacity":0.5}))]
    );
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
