//! One round trip per shorthand: the source line and the props it must write.
use super::keys::effective_key;
use super::tests::ops;
use opencanvas_storage::prop_spec;
use serde_json::{json, Value};

/// Attribute shorthands, resolved through the same lookup the compiler uses.
const ATTRIBUTES: [&str; 12] = [
    "w", "h", "r", "p", "gap", "align", "justify", "stroke", "size", "font", "family", "weight",
];
/// Keys written by the other shorthands: WxH, flags, `.style`, `size=a/b`, comp and use.
const WRITTEN: [&str; 8] = [
    "width",
    "height",
    "layout",
    "styleId",
    "lineHeight",
    "component",
    "componentId",
    "overrides",
];

/// Props of the last created node of `page\n  <line>` (or of `@set` when `line` starts with @).
fn props(line: &str) -> Value {
    let source = match line.starts_with('@') {
        true => line.to_owned(),
        false => format!("page#p\n  frame#host 100x100\n    {line}\n"),
    };
    ops(&source).pop().unwrap()["props"].clone()
}

#[test]
fn every_shorthand_target_is_a_catalog_key() {
    for shorthand in ATTRIBUTES {
        let target = effective_key(shorthand);
        assert!(target.and_then(prop_spec).is_some(), "{shorthand}");
    }
    for target in WRITTEN {
        assert!(prop_spec(target).is_some(), "{target}");
    }
}

#[test]
fn size_and_dimensions() {
    assert_eq!(props("rect 400x44"), json!({"width":400,"height":44}));
    assert_eq!(props("rect 1.5x2"), json!({"width":1.5,"height":2}));
    assert_eq!(
        props("rect w=fill h=50%"),
        json!({"width":"fill","height":"50%"})
    );
    assert_eq!(
        props("rect w=$wide h=hug"),
        json!({"width":{"tokenId":"wide"},"height":"hug"})
    );
}

#[test]
fn radius() {
    assert_eq!(props("rect r=8"), json!({"radius":8}));
    assert_eq!(props("rect r=8,8,0,0"), json!({"radius":[8,8,0,0]}));
}

#[test]
fn layout_flags_and_keys() {
    assert_eq!(props("frame row"), json!({"layout":{"mode":"row"}}));
    assert_eq!(props("frame col"), json!({"layout":{"mode":"column"}}));
    assert_eq!(props("frame free"), json!({"layout":{"mode":"free"}}));
    assert_eq!(
        props("frame row wrap"),
        json!({"layout":{"mode":"row","wrap":true}})
    );
    assert_eq!(
        props("frame col p=0,12 gap=$g align=stretch justify=between"),
        json!({"layout":{"mode":"column","padding":[0,12],"gap":{"tokenId":"g"},"align":"stretch","justify":"space-between"}})
    );
    assert_eq!(props("frame p=32"), json!({"layout":{"padding":32}}));
}

#[test]
fn style_and_token_references() {
    assert_eq!(props("rect .card"), json!({"styleId":"card"}));
    assert_eq!(props("rect.card"), json!({"styleId":"card"}));
    assert_eq!(props("rect fill=$ink"), json!({"fill":{"tokenId":"ink"}}));
}

/// `.style` takes every ID storage and `@style` accept; only a complete `.5x10` is a size.
#[test]
fn dotted_styles_take_any_valid_id() {
    for (dotted, explicit) in [
        ("rect .123", "rect styleId=123"),
        ("rect .-ink", "rect styleId=-ink"),
        ("rect.123", "rect styleId=123"),
        ("rect#r.-ink fill=#000", "rect#r styleId=-ink fill=#000"),
        ("text .5", "text styleId=5"),
    ] {
        assert_eq!(props(dotted), props(explicit), "{dotted}");
    }
    assert_eq!(props("rect .123"), json!({"styleId":"123"}));
    assert_eq!(props("@set a .-ink"), json!({"styleId":"-ink"}));
    assert_eq!(props("rect .5x10"), json!({"width":0.5,"height":10}));
    for (source, column) in [("page#p\n  rect .\n", 8), ("page#p\n  rect.\n", 7)] {
        let e = super::compile(source).expect_err(source);
        assert_eq!((e.line, e.column), (2, column), "{source}");
        assert!(e.message.contains(".style needs an ID"), "{}", e.message);
    }
}

#[test]
fn stroke() {
    assert_eq!(
        props("rect stroke=#E5E7EB,2,inside"),
        json!({"stroke":{"color":"#E5E7EB","width":2,"align":"inside"}})
    );
    assert_eq!(
        props("rect stroke=$line"),
        json!({"stroke":{"color":{"tokenId":"line"}}})
    );
    assert_eq!(
        props(r##"rect stroke={"color":["#000","#fff"],"width":[1,2]}"##),
        json!({"stroke":{"color":["#000","#fff"],"width":[1,2]}})
    );
}

#[test]
fn text_metrics_and_fonts() {
    let text = props("text size=24/32 font=font_bundled weight=700 \"Hi\"");
    assert_eq!(
        text,
        json!({"text":"Hi","fontSize":24,"lineHeight":32,"fontId":"font_bundled","fontWeight":700,"width":"hug","height":"hug"})
    );
    assert_eq!(props("@set t size=28"), json!({"fontSize":28}));
    assert_eq!(props("@set t family=Inter"), json!({"fontFamily":"Inter"}));
    assert_eq!(
        props("@set t text=2024 name=42"),
        json!({"text":"2024","name":"42"})
    );
}

#[test]
fn component_and_instance() {
    assert_eq!(
        props("comp#btn 10x10"),
        json!({"component":{"name":"btn"},"width":10,"height":10})
    );
    assert_eq!(
        props("comp#btn name=Button"),
        json!({"component":{"name":"Button"}})
    );
    assert_eq!(
        props("use#a btn label=\"Go\" label.fill=$pri icon={\"visible\":false} w=120 name=Primary"),
        json!({"componentId":"btn","width":120,"name":"Primary",
            "overrides":{"label":{"text":"Go","fill":{"tokenId":"pri"}},"icon":{"visible":false}}})
    );
    assert_eq!(
        props("use#a btn overrides={\"a/b\":{\"text\":\"x\"}}"),
        json!({"componentId":"btn","overrides":{"a/b":{"text":"x"}}})
    );
}

#[test]
fn raw_json_props_merge_last_and_catalog_keys_pass_through() {
    assert_eq!(
        props(
            r##"rect fill=#000 opacity=0.5 clip=true props={"fill":{"type":"linear","stops":[[0,"#000"],[1,"#fff"]]}}"##
        ),
        json!({"fill":{"type":"linear","stops":[[0,"#000"],[1,"#fff"]]},"opacity":0.5,"clip":true})
    );
    assert_eq!(
        props(r#"frame layout={"mode":"row","gap":4} states={"hover":{"opacity":0.8}}"#),
        json!({"layout":{"mode":"row","gap":4},"states":{"hover":{"opacity":0.8}}})
    );
    let op = ops("page#p\n  frame#a 10x10\n    rect#r index=0\n")
        .pop()
        .unwrap();
    assert_eq!(
        (op["index"].clone(), op["props"].clone()),
        (json!(0), json!({}))
    );
}
