use opencanvas_storage::{BatchRequest, Snapshot, Store};
use serde_json::{json, Value};

pub fn fixture(pages: usize) -> Snapshot {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path()).unwrap();
    let project = store.create_project("find-fixture", "Find").unwrap();
    let create = |id: &str, kind: &str, parent: Option<&str>, props: Value| json!({"op":"create_node","id":id,"kind":kind,"parentId":parent,"props":props});
    let text = |copy: String| json!({"name":"Needle text","text":copy,"fontId":"font_bundled","fontSize":14,"lineHeight":20,"width":100,"height":20,"fill":"#111111"});
    let mut ops = Vec::new();
    for p in 0..pages {
        let page = format!("page{p}");
        let frame = format!("frame{p}");
        ops.push(create(&page, "page", None, json!({"name":page})));
        ops.push(create(
            &frame,
            "frame",
            Some(&page),
            json!({"name":"Needle frame","width":300,"height":300}),
        ));
        for t in 0..12 {
            ops.push(create(
                &format!("text{p}-{t}"),
                "text",
                Some(&frame),
                text(format!("{}NEEDLE {p}/{t} Ω", "á日🙂".repeat(65))),
            ));
        }
    }
    ops.push(create(
        "master",
        "frame",
        Some("page0"),
        json!({"width":200,"height":200,"component":{"name":"Master"}}),
    ));
    ops.push(create(
        "master-text",
        "text",
        Some("master"),
        text("Needle original".into()),
    ));
    ops.push(create(
        "nested-master",
        "frame",
        Some("page0"),
        json!({"width":200,"height":200,"component":{"name":"Nested"}}),
    ));
    ops.push(create(
        "nested",
        "instance",
        Some("nested-master"),
        json!({"componentId":"master","overrides":{"master-text":{"text":"Needle override"}}}),
    ));
    ops.push(create("placed", "instance", Some(&format!("frame{}", pages - 1)), json!({"name":"Needle instance","componentId":"nested-master","overrides":{"nested":{"name":"Needle nested"}}})));
    let mut parent = "frame0".to_owned();
    for d in 0..11 {
        let id = format!("deep{d}");
        ops.push(create(
            &id,
            "group",
            Some(&parent),
            json!({"name":"Needle deep","width":100,"height":100}),
        ));
        parent = id;
    }
    ops.push(create(
        "deep-text",
        "text",
        Some(&parent),
        text("Needle deep text".into()),
    ));
    let request: BatchRequest = serde_json::from_value(json!({"projectId":project.id(),"expectedRevision":0,"idempotencyKey":"find","operations":ops})).unwrap();
    project.apply_batch(request).unwrap();
    let mut snapshot = project.snapshot().unwrap();
    snapshot.project_id = "p_find_reference".into();
    snapshot.nodes.reverse();
    for node in &mut snapshot.nodes {
        node.sort_key = 7;
    }
    snapshot
}
