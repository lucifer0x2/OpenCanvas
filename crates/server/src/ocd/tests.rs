//! The published sample against its checked-in golden operations, plus directive output.
use super::compile;
use serde_json::{json, Value};

const GUIDE: &str = include_str!("../../skills/ocd.md");
const GOLDEN: &str = include_str!("../../tests/fixtures/ocd-login.json");

/// The `ocd` block after `<!-- marker -->` in the published guide.
pub fn sample(marker: &str) -> &'static str {
    let (_, after) = GUIDE.split_once(&format!("<!-- {marker} -->")).unwrap();
    let (_, after) = after.split_once("```ocd\n").unwrap();
    after.split_once("```").unwrap().0
}

pub fn ops(source: &str) -> Vec<Value> {
    match compile(source) {
        Ok(compiled) => compiled.operations,
        Err(e) => panic!("{source}: {e:?}"),
    }
}

#[test]
fn guide_sample_compiles_to_the_golden_operations() {
    assert!(GUIDE.len() <= 4096, "ocd.md is {} bytes", GUIDE.len());
    let golden: Value = serde_json::from_str(GOLDEN).unwrap();
    assert_eq!(Value::Array(ops(sample("ocd-sample"))), golden);
}

#[test]
fn guide_edit_compiles_to_a_replace_a_merge_and_a_delete() {
    assert_eq!(
        ops(sample("ocd-edit")),
        vec![
            json!({"op":"update_node","id":"title","props":{"text":"Chào mừng trở lại","fontSize":28}}),
            json!({"op":"update_node","id":"card","props":{"layout":{"gap":24}},"merge":true}),
            json!({"op":"delete_subtree","id":"pass"}),
        ]
    );
}

#[test]
fn auto_ids_are_anchored_hashed_reported_and_comments_are_ignored() {
    let source = "// header\npage#pg\n  frame#a 10x10\n\n  frame 10x10 // second\n    rect\n    text \"Hi\"\n";
    let compiled = compile(source).unwrap();
    let ids: Vec<_> = compiled
        .operations
        .iter()
        .map(|op| op["id"].as_str().unwrap())
        .collect();
    let (frame, rect, text) = (ids[2], ids[3], ids[4]);
    assert!(
        frame.starts_with("pg-frame-") && frame.len() == "pg-frame-".len() + 8,
        "{frame}"
    );
    // Below a generated ID the anchor stays the nearest written `#id`.
    assert!(
        rect.starts_with("pg-rect-") && text.starts_with("pg-text-"),
        "{ids:?}"
    );
    assert_eq!(compiled.operations[3]["parentId"], frame);
    let lines: Vec<_> = compiled
        .auto_ids
        .iter()
        .map(|(line, id)| (*line, id.as_str()))
        .collect();
    assert_eq!(lines, [(5, frame), (6, rect), (7, text)]);
    // The same source compiles to the same IDs; a sibling added before moves none.
    assert_eq!(compile(source).unwrap().operations, compiled.operations);
    let moved = source.replace("    rect\n", "    image\n    rect\n");
    let again = ops(&moved);
    assert_eq!(
        (again[4]["id"].as_str(), again[5]["id"].as_str()),
        (Some(rect), Some(text))
    );
}

#[test]
fn every_operation_keeps_its_statement_position() {
    let source = "@token ink #111\npage#p\n  frame#a 10x10\n  @in a\n    rect#r\n@del x\n";
    let compiled = compile(source).unwrap();
    assert_eq!(compiled.positions, [(1, 1), (2, 1), (3, 3), (5, 5), (6, 1)]);
}

#[test]
fn tokens_styles_moves_notes_and_in() {
    let source = "@token ink #111 dark=#EEE name=Ink\n@token gap 8\n@token alias $ink type=color\n\
        @style accent fill=$ink\n@style label family=Inter weight=600 size=12 name=\"Label\"\n\
        @move a b 2\n@note a id=n1 \"why\"\n@in card\n  text#t font=font_bundled size=12/16 \"Hi\"\n";
    assert_eq!(
        ops(source),
        vec![
            json!({"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":{"default":"#111","themes":{"dark":"#EEE"}}}}),
            json!({"op":"upsert_token","token":{"id":"gap","name":"gap","type":"dimension","value":8}}),
            json!({"op":"upsert_token","token":{"id":"alias","name":"alias","type":"color","value":{"aliasId":"ink"}}}),
            json!({"op":"upsert_style","style":{"id":"accent","name":"accent","kind":"fill","value":{"tokenId":"ink"}}}),
            json!({"op":"upsert_style","style":{"id":"label","name":"Label","kind":"text","value":{"fontFamily":"Inter","fontWeight":600,"fontSize":12}}}),
            json!({"op":"reparent_node","id":"a","parentId":"b","index":2}),
            json!({"op":"create_note","id":"n1","nodeId":"a","text":"why"}),
            // Under @in the stored parent's layout is unknown: no size defaults.
            json!({"op":"create_node","id":"t","kind":"text","parentId":"card","props":{"text":"Hi","fontId":"font_bundled","fontSize":12,"lineHeight":16}}),
        ]
    );
}

/// `(width, height)` of each text child of the frames in `frames` (one per line).
fn text_sizes(frames: &[&str]) -> Vec<(Value, Value)> {
    let mut source = "page#p\n".to_owned();
    for frame in frames {
        source += &format!("  {frame}\n    text\n");
    }
    ops(&source)
        .iter()
        .filter(|op| op["kind"] == "text")
        .map(|op| (op["props"]["width"].clone(), op["props"]["height"].clone()))
        .collect()
}

#[test]
fn text_size_defaults_never_defeat_the_parent_alignment() {
    let (fill, hug, none) = (json!("fill"), json!("hug"), Value::Null);
    let cases = [
        ("frame 100x100 col", (&fill, &hug)),
        ("frame 100x100 col align=start", (&fill, &hug)),
        ("frame h=10 col", (&hug, &hug)),
        ("frame 100x100 col align=stretch", (&none, &hug)),
        ("frame h=10 col align=stretch", (&none, &hug)),
        ("frame 100x100 col align=center", (&hug, &hug)),
        ("frame 100x100 col align=end", (&hug, &hug)),
        (
            r#"frame 100x100 layout={"mode":"column","align":"center"}"#,
            (&hug, &hug),
        ),
        ("frame 100x10 row", (&hug, &hug)),
        ("frame 100x10 row align=stretch", (&hug, &none)),
        ("frame 100x10", (&hug, &hug)),
    ];
    let frames: Vec<_> = cases.iter().map(|(frame, _)| *frame).collect();
    let expected: Vec<_> = cases
        .iter()
        .map(|(_, (w, h))| ((*w).clone(), (*h).clone()))
        .collect();
    assert_eq!(text_sizes(&frames), expected);
    // An authored size always wins.
    let written = ops("page#p\n  frame 100x100 col align=stretch\n    text w=20 h=fill\n");
    assert_eq!(written[2]["props"], json!({"width":20,"height":"fill"}));
}

/// `frame 100x100` whose layout is `base` plus one breakpoint rule `rule` (raw JSON).
fn responsive(base: &str, rule: &str) -> String {
    format!(r#"frame 100x100 layout={{{base},"breakpoints":[{{"maxWidth":600,"layout":{rule}}}]}}"#)
}

#[test]
fn text_size_defaults_hold_for_every_breakpoint_layout() {
    let (fill, hug, none) = (json!("fill"), json!("hug"), Value::Null);
    let column = r#""mode":"column""#;
    let cases = [
        // A stretching breakpoint must keep the width unset.
        (
            responsive(
                r#""mode":"column","align":"center""#,
                r#"{"mode":"column","align":"stretch"}"#,
            ),
            (&none, &hug),
        ),
        (
            responsive(column, r#"{"mode":"column","align":"start"}"#),
            (&fill, &hug),
        ),
        // Disagreeing rules: fill (column) against hug (row) leaves the width unset.
        (responsive(column, r#"{"mode":"row"}"#), (&none, &hug)),
        (
            responsive(r#""mode":"row","align":"stretch""#, r#"{"mode":"column"}"#),
            (&none, &none),
        ),
        (
            responsive(r#""mode":"row""#, r#"{"mode":"row","align":"center"}"#),
            (&hug, &hug),
        ),
        // Malformed breakpoints: storage reports them, so no default is guessed.
        (responsive(column, r#""column""#), (&none, &none)),
        (
            format!(r#"frame 100x100 layout={{{column},"breakpoints":{{}}}}"#),
            (&none, &none),
        ),
        (
            format!(r#"frame 100x100 layout={{{column},"breakpoints":[1]}}"#),
            (&none, &none),
        ),
    ];
    let frames: Vec<_> = cases.iter().map(|(frame, _)| frame.as_str()).collect();
    let expected: Vec<_> = cases
        .iter()
        .map(|(_, (w, h))| ((*w).clone(), (*h).clone()))
        .collect();
    assert_eq!(text_sizes(&frames), expected);
}
