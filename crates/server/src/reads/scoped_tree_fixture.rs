//! Real committed temporary design for scoped/full read comparisons.
use opencanvas_storage::{BatchRequest, Snapshot, Storage, Store};
use serde_json::{json, Value};

pub(super) fn node(id: &str, kind: &str, parent: Option<&str>, props: Value) -> Value {
    json!({"op":"create_node","id":id,"kind":kind,"parentId":parent,"props":props})
}

pub(super) fn fixture() -> (tempfile::TempDir, Storage, String) {
    let dir = tempfile::tempdir().unwrap();
    let storage: Storage = Store::open(dir.path()).unwrap().into();
    let project = storage
        .create_project("scoped-tree", "Scoped tree")
        .unwrap();
    let mut ops = vec![
        json!({"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":{"default":"#111111","themes":{"night":"#eeeeee","day":"#008800"}}}}),
        json!({"op":"upsert_token","token":{"id":"unused","name":"Unused","type":"number","value":123}}),
        json!({"op":"upsert_style","style":{"id":"paint","name":"Paint","kind":"fill","value":{"tokenId":"ink"}}}),
        node("page", "page", None, json!({"name":"Main page"})),
        node("library", "page", None, json!({"name":"Library"})),
        node(
            "frame",
            "frame",
            Some("page"),
            json!({"name":"Main","theme":"day","width":320,"height":200,"flowStart":"App"}),
        ),
        node(
            "definitions",
            "frame",
            Some("library"),
            json!({"name":"Definitions","theme":"night","width":320,"height":200}),
        ),
        node(
            "master",
            "frame",
            Some("definitions"),
            json!({"name":"Master","component":{"name":"Tile"},"width":100,"height":100}),
        ),
        node(
            "box",
            "rectangle",
            Some("master"),
            json!({"width":20,"height":20,"fill":{"styleId":"paint"},"states":{"hover":{"fill":{"tokenId":"ink"}}}}),
        ),
        node(
            "nested-master",
            "frame",
            Some("definitions"),
            json!({"component":{"name":"Label"},"width":100,"height":40}),
        ),
        node(
            "copy",
            "text",
            Some("nested-master"),
            json!({"width":100,"height":20,"text":"Tổng quan","fontId":"font_bundled","fontSize":14,"lineHeight":20,"fill":{"tokenId":"ink"}}),
        ),
        node(
            "nested",
            "instance",
            Some("master"),
            json!({"componentId":"nested-master"}),
        ),
        node(
            "placed",
            "instance",
            Some("frame"),
            json!({"componentId":"master","theme":"default","overrides":{"box":{"fill":"#ff0000"}}}),
        ),
        node(
            "incoming",
            "frame",
            Some("library"),
            json!({"name":"Incoming","width":100,"height":100}),
        ),
        node(
            "outside",
            "instance",
            Some("incoming"),
            json!({"componentId":"master"}),
        ),
        node(
            "button",
            "rectangle",
            Some("incoming"),
            json!({"width":20,"height":20,"links":[{"to":"frame"}]}),
        ),
        node(
            "outgoing",
            "rectangle",
            Some("frame"),
            json!({"width":20,"height":20,"links":[{"to":"incoming"},{"action":"overlay","on":"hover","to":"incoming"}],"animations":[{"name":"fade","duration":200,"keyframes":[{"opacity":0.5}]}]}),
        ),
    ];
    for (id, anchor) in [
        ("note-page", "page"),
        ("note-frame", "frame"),
        ("note-placed", "placed"),
        ("note-copy", "copy"),
    ] {
        ops.push(json!({"op":"create_note","id":id,"nodeId":anchor,"text":"Intentional lettering: example","author":"designer"}));
    }
    ops.push(json!({"op":"update_note","id":"note-placed","resolved":true}));
    project.apply_batch(serde_json::from_value::<BatchRequest>(json!({"projectId":project.id(),"expectedRevision":0,"idempotencyKey":"fixture","operations":ops})).unwrap()).unwrap();
    (dir, storage, project.id().to_owned())
}

pub(super) fn reordered(snapshot: &Snapshot) -> Snapshot {
    let mut snapshot = snapshot.clone();
    snapshot.nodes.reverse();
    for node in &mut snapshot.nodes {
        node.sort_key = 3;
    }
    snapshot
}
