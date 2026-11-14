//! Note anchors and page appends: indexed footprints match the frozen reference.
use super::*;

fn keys(before: &Snapshot, case: Value) -> (Snapshot, BTreeSet<String>) {
    step(before, serde_json::from_value(case).unwrap())
}

#[test]
fn note_anchor_pages_and_local_page_appends_match_reference() {
    let before = seed();
    let page = |id: &str| json!({"op":"create_node","id":id,"kind":"page","parentId":null,"props":{"name":id}});
    let frame = json!({"op":"create_node","id":"nf","kind":"frame","parentId":"tail-page","props":{"width":10,"height":10}});
    let (_, appended) = keys(&before, json!([page("tail-page"), frame]));
    assert!(
        !appended.contains("*") && appended.contains("page:tail-page"),
        "{appended:?}"
    );
    assert!(
        !appended.iter().any(|k| k.starts_with("page:p")),
        "{appended:?}"
    );
    let mut front = page("front");
    front["index"] = json!(0);
    assert!(keys(&before, json!([front])).1.contains("*"));
    assert!(keys(
        &before,
        json!([{"op":"update_node","id":"p1","props":{"name":"R"}}])
    )
    .1
    .contains("*"));

    let (_, created) = keys(
        &before,
        json!([{"op":"create_note","id":"n2","nodeId":"r1","text":"x"}]),
    );
    assert_eq!(created, ["note:n2", "page:p1"].map(String::from).into());
    let (_, moved) = keys(
        &before,
        json!([{"op":"update_note","id":"note","nodeId":"r1"}]),
    );
    assert_eq!(
        moved,
        ["note:note", "page:p1", "page:p2"].map(String::from).into()
    );
    let (_, deleted) = keys(&before, json!([{"op":"delete_note","id":"note"}]));
    assert_eq!(deleted, ["note:note", "page:p2"].map(String::from).into());

    // Deleting `p2` leaves a sort-key gap; the next append compacts `library`
    // without naming its page.
    let (gapped, _) = keys(&before, json!([{"op":"delete_subtree","id":"p2"}]));
    let (_, compacted) = keys(&gapped, json!([page("after-gap")]));
    assert!(compacted.contains("node:library") && !compacted.contains("page:library"));
    assert!(!compacted.contains("*"), "{compacted:?}");
}
