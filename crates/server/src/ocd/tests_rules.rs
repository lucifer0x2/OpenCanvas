//! Review findings on Task26: linear lists, `@in`, generated IDs, descendant shorthands,
//! `@style` keys and parser edges.
use super::notes::note_id;
use super::tests::ops;
use super::{compile, OcdError};
use serde_json::json;
use std::time::{Duration, Instant};

fn error(source: &str) -> OcdError {
    compile(source).expect_err(source)
}

fn at(source: &str) -> (usize, usize, String) {
    let e = error(source);
    (e.line, e.column, e.message)
}

#[test]
fn long_lists_compile_in_linear_time() {
    let items = 200_000;
    let padding = format!("page#p\n  frame#f p={}1\n", "1,".repeat(items - 1));
    let started = Instant::now();
    let compiled = compile(&padding).unwrap();
    let elapsed = started.elapsed();
    let padding = &compiled.operations[1]["props"]["layout"]["padding"];
    assert_eq!(padding.as_array().unwrap().len(), items);
    assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
    let size = format!("page#p\n  text size={}1\n", "1/".repeat(items - 1));
    let started = Instant::now();
    assert_eq!(at(&size).0, 2);
    let elapsed = started.elapsed();
    assert!(elapsed < Duration::from_secs(2), "{elapsed:?}");
}

#[test]
fn in_takes_exactly_one_parent_id_and_needs_children() {
    let (line, column, message) = at("@in page-1-1 row gap=8 fill=#ff0000\n  rect#r\n");
    assert_eq!((line, column), (1, 14));
    assert!(
        message.starts_with("@in takes only a parent ID"),
        "{message}"
    );
    let (line, column, _) = at("@in card \"hello\"\n");
    assert_eq!((line, column), (1, 10));
    let (line, column, message) = at("@del a\n@in card\n");
    assert_eq!(
        (line, column, message.as_str()),
        (2, 1, "@in needs indented child nodes")
    );
    assert_eq!(at("@in a\n@in b\n  rect#r\n").0, 1);
}

#[test]
fn root_pages_need_an_id() {
    let (line, column, message) = at("page\n  frame#a 1x1\n");
    assert_eq!((line, column), (1, 1));
    assert!(message.starts_with("A root page needs #id"), "{message}");
}

#[test]
fn default_note_ids_depend_on_node_and_text_and_stay_valid() {
    let first = ops("@note card \"why\"")[0]["id"].clone();
    // Same note on another line, in another batch: same ID; other text: another ID.
    assert_eq!(ops("@del x\n\n@note card \"why\"")[1]["id"], first);
    assert_ne!(ops("@note card \"why not\"")[0]["id"], first);
    assert!(first.as_str().unwrap().starts_with("note-card-"), "{first}");
    let long = "n".repeat(128);
    let id = note_id(&long, "why");
    assert!(id.len() <= 128 && id.starts_with("note-nnn"), "{id}");
    assert_ne!(id, note_id(&long, "other"));
    assert_eq!(ops("@note card id=n1 \"why\"")[0]["id"], "n1");
}

#[test]
fn dotted_descendant_keys_use_node_shorthands() {
    let source = "page#p\n  use#a input ph.r=8 ph.stroke=#fff,2 ph.fill=$pri ph.text=42 ph.p=4\n";
    assert_eq!(
        ops(source)[1]["props"]["overrides"],
        json!({"ph":{"radius":8,"stroke":{"color":"#fff","width":2},
            "fill":{"tokenId":"pri"},"text":"42","layout":{"padding":4}}})
    );
    let e = error("page#p\n  use#a input ph.color=1\n");
    assert_eq!(
        (e.line, e.column, e.message.as_str()),
        (2, 18, "Unknown key `color`")
    );
    assert_eq!(e.suggestion.as_deref(), Some("fill"));
}

#[test]
fn style_rejects_dotted_and_non_style_raw_keys() {
    let (line, column, message) = at("@style s ph.fill=#fff\n");
    assert_eq!((line, column), (1, 10));
    assert!(
        message.contains("`ph.fill` is not a fill or text style"),
        "{message}"
    );
    let (_, column, message) = at(r#"@style s props={"width":1}"#);
    assert_eq!(column, 16);
    assert!(
        message.contains("`width` is not a fill or text style"),
        "{message}"
    );
    assert_eq!(
        ops(r##"@style s props={"fill":"#000"}"##)[0]["style"]["kind"],
        "fill"
    );
}

#[test]
fn parser_edges() {
    let (line, column, message) = at("page#p\n  \u{a0}frame#a\n");
    assert_eq!((line, column), (2, 3));
    assert!(message.contains("U+00A0"), "{message}");
    for name in [r#"name={"a":1}"#, "name=$t", "name=[1]"] {
        let (_, column, message) = at(&format!("page#p\n  comp#x {name}\n"));
        assert_eq!(column, 15, "{name}");
        assert!(message.starts_with("A component name is text"), "{message}");
    }
    assert_eq!(
        ops("@token ink \"#111\"")[0]["token"],
        json!({"id":"ink","name":"ink","type":"color","value":"#111"})
    );
    let (_, _, message) = at("@token ink\n");
    assert!(message.contains("@token id value"), "{message}");
    assert_eq!(
        ops("page#p\n  frame#f\n    rect .5x10\n")[2]["props"],
        json!({"width":0.5,"height":10})
    );
    // The head form has no size: `rect.5x10` is the style `5x10` (see tests_shorthands).
    assert_eq!(
        ops("page#p\n  rect.5x10\n")[1]["props"],
        json!({"styleId":"5x10"})
    );
    let (line, column, message) = at("  page#p\n");
    assert_eq!(
        (line, column, message.as_str()),
        (1, 3, "The first line must not be indented")
    );
    assert!(at("@del a\n  rect#r\n")
        .2
        .starts_with("Indented under @del"));
}

#[test]
fn source_cap_counts_utf8_bytes() {
    let text = "Đ".repeat(1_100_000);
    let source = format!("page#p\n  text \"{text}\"\n");
    assert!(source.chars().count() < 2 * 1024 * 1024);
    let (line, _, message) = at(&source);
    assert_eq!(line, 2);
    assert!(message.contains("UTF-8 bytes"), "{message}");
}
