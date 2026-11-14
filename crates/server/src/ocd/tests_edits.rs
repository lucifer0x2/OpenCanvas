//! Note directives (`@note … author=`, `@reply`, `@note-set`, `@resolve`, `@note-del`),
//! `@order`, and override targets on `use` (typos fail with a suggestion).
use super::tests::ops;
use super::{compile, OcdError};
use serde_json::json;

fn error(source: &str) -> OcdError {
    compile(source).expect_err(source)
}

#[test]
fn notes_take_an_author_and_text_on_either_side() {
    let expected =
        json!({"op":"create_note","id":"n1","nodeId":"card","text":"why","author":"Ann"});
    for source in [
        "@note card \"why\" author=Ann id=n1",
        "@note card id=n1 author=\"Ann\" \"why\"",
    ] {
        assert_eq!(ops(source), vec![expected.clone()], "{source}");
    }
    // The default ID depends on node and text only.
    let plain = &ops("@note card \"why\"")[0]["id"];
    assert_eq!(&ops("@note card \"why\" author=Ann")[0]["id"], plain);
}

#[test]
fn notes_can_be_replied_to_edited_resolved_and_deleted() {
    let source = "@reply n1 \"ok\" author=Bo\n@note-set n1 \"new\" resolved=false node=btn\n\
        @note-set n1 resolved=true\n@resolve n1\n@note-del n1\n@order btn 0\n";
    assert_eq!(
        ops(source),
        vec![
            json!({"op":"reply_note","id":"n1","text":"ok","author":"Bo"}),
            json!({"op":"update_note","id":"n1","text":"new","resolved":false,"nodeId":"btn"}),
            json!({"op":"update_note","id":"n1","resolved":true}),
            json!({"op":"update_note","id":"n1","resolved":true}),
            json!({"op":"delete_note","id":"n1"}),
            json!({"op":"reorder_node","id":"btn","index":0}),
        ]
    );
}

#[test]
fn bad_note_and_order_lines_point_at_the_problem() {
    let cases = [
        (
            "@note card \"why\" by=Ann",
            (1, 18),
            "@note takes \"text\" and id=, author=",
        ),
        ("@reply n1", (1, 2), "@reply needs a quoted text"),
        (
            "@reply n1 \"ok\" id=x",
            (1, 16),
            "@reply takes \"text\" and author=",
        ),
        ("@note-set n1", (1, 2), "@note-set needs"),
        (
            "@note-set n1 resolved=yes",
            (1, 23),
            "resolved takes true or false",
        ),
        ("@note-set n1 node=a/b", (1, 19), "Invalid ID"),
        ("@resolve n1 \"x\"", (1, 13), "@resolve takes only IDs"),
        ("@note-del", (1, 1), "@note-del needs 1 argument"),
        ("@order btn", (1, 1), "@order needs 2 argument"),
        ("@order btn first", (1, 12), "index takes an integer"),
        ("@frob x", (1, 1), "@order"),
    ];
    for (source, at, message) in cases {
        let e = error(source);
        assert_eq!((e.line, e.column), at, "{source}: {}", e.message);
        assert!(e.message.contains(message), "{source}: {}", e.message);
    }
}

const MASTER: &str = "page#p\n  comp#input 10x10\n    text#ph \"x\"\n    rect#icon 1x1\n";

#[test]
fn use_keys_must_be_instance_props_or_master_descendants() {
    let ok = format!("{MASTER}  use#a input w=20 ph=\"y\" icon.fill=#fff name=A\n");
    assert_eq!(
        ops(&ok)[4]["props"]["overrides"],
        json!({"ph":{"text":"y"},"icon":{"fill":"#fff"}})
    );
    for (line, column, suggestion) in [
        ("  use#a input pj=\"y\"", 15, Some("ph")),
        ("  use#a input icn.fill=#fff", 15, Some("icon")),
        ("  use#a input widht=20", 15, None),
        ("  use#a input wdth=20", 15, Some("width")),
    ] {
        let e = error(&format!("{MASTER}{line}\n"));
        assert_eq!((e.line, e.column), (5, column), "{line}: {}", e.message);
        assert!(e.message.contains("nor a descendant"), "{}", e.message);
        assert_eq!(e.suggestion.as_deref(), suggestion, "{line}");
    }
    // A node prop that instances do not take, with the master known or not.
    for source in [
        format!("{MASTER}  use#a input text=\"y\"\n"),
        "page#p\n  use#a input gap=8\n".into(),
    ] {
        let e = error(&source);
        assert!(
            e.message.contains("is not an instance property"),
            "{}",
            e.message
        );
    }
    // A stored master is checked by storage: any other key compiles as a target.
    assert_eq!(
        ops("page#p\n  use#a stored titel=\"y\"\n")[1]["props"]["overrides"],
        json!({"titel":{"text":"y"}})
    );
}
