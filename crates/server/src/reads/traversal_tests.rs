//! Byte-for-byte guard for the outline/subtree/context traversal. The golden file was written by
//! the pre-index `visit()` implementation and re-baselined once by B03, when depth became a filter
//! from the scope root (whole-scope pages were unchanged); set OPENCANVAS_WRITE_GOLDEN=1 only to
//! re-baseline.
use super::{index::TreeIndex, pagination, tree, viewports::Computed, ReadRequest};
use opencanvas_storage::{BatchRequest, Node, Snapshot, Store};
use serde_json::{json, Value};
use std::collections::BTreeMap;

const GOLDEN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/traversal_golden.jsonl"
);

/// Valid stored design, then storage order reversed (every parent after its children) and
/// duplicate sibling sort keys, so a stable sort over storage order is what decides ties.
fn fixture() -> Snapshot {
    let root = tempfile::tempdir().unwrap();
    let store = Store::open(root.path()).unwrap();
    let project = store.create_project("fixture", "Fixture").unwrap();
    let text = |id: &str, parent: &str, copy: &str| json!({"op":"create_node","id":id,"kind":"text","parentId":parent,"props":{"name":id,"text":copy,"width":120,"height":"hug","fontId":"font_bundled","fontSize":14,"lineHeight":20,"fill":{"tokenId":"ink"}}});
    let group = |id: &str, parent: &str| json!({"op":"create_node","id":id,"kind":"group","parentId":parent,"props":{"name":id,"width":200,"height":80,"fill":"#eeeeee"}});
    let operations = json!([
        {"op":"upsert_token","token":{"id":"ink","name":"Ink","type":"color","value":{"default":"#111111","themes":{"night":"#eeeeee"}}}},
        {"op":"create_node","id":"page-b","kind":"page","parentId":null,"props":{"name":"B"}},
        {"op":"create_node","id":"page-a","kind":"page","parentId":null,"props":{"name":"A"}},
        {"op":"create_node","id":"frame","kind":"frame","parentId":"page-a","props":{"name":"Main","width":400,"height":300,"fill":{"tokenId":"ink"}}},
        group("z-card", "frame"), group("a-card", "frame"), group("m-card", "frame"),
        text("child-text", "z-card", "Tổng quan"), text("a-text", "a-card", "Second"),
        group("deep", "a-card"), text("deep-text", "deep", "Deep copy"),
        {"op":"create_node","id":"frame-b","kind":"frame","parentId":"page-b","props":{"width":100,"height":100,"layout":{"mode":"row","padding":0,"gap":4}}},
        {"op":"create_node","id":"rect","kind":"rectangle","parentId":"frame-b","props":{"width":10,"height":10}}
    ]);
    let request: BatchRequest = serde_json::from_value(json!({"projectId":project.id(),"expectedRevision":0,"idempotencyKey":"fixture","operations":operations})).unwrap();
    project.apply_batch(request).unwrap();
    let mut s = project.snapshot().unwrap();
    s.project_id = "p_fixture".into();
    s.nodes.reverse();
    for node in &mut s.nodes {
        node.sort_key = match node.id.as_str() {
            "page-a" | "page-b" => 7,
            "z-card" | "a-card" => 3,
            "a-text" | "deep" => 0,
            _ => node.sort_key,
        };
    }
    s
}

fn computed(s: &Snapshot) -> Computed {
    let bounds: BTreeMap<String, Vec<Value>> = s
        .nodes
        .iter()
        .map(|n| {
            (
                n.id.clone(),
                vec![json!({"x":1,"y":2,"width":3,"height":4})],
            )
        })
        .collect();
    Computed {
        metadata: json!({"status":"ready","viewports":[]}),
        bounds,
        reflow: Some(false),
    }
}

/// Every page of one read, following nextCursor with identical parameters.
fn pages(s: &Snapshot, tool: &str, args: Value, with_bounds: bool) -> Vec<String> {
    let mut out = Vec::new();
    let mut args = args;
    loop {
        let req: ReadRequest = serde_json::from_value(args.clone()).unwrap();
        let (scope, offset) = pagination::scope(s, tool, &req).unwrap();
        let bounds = with_bounds.then(|| computed(s));
        let page = tree::read(s, tool, req, scope, offset, bounds).unwrap();
        out.push(serde_json::to_string(&page).unwrap());
        if page["truncated"] != true {
            return out;
        }
        args["cursor"] = page["nextCursor"].clone();
    }
}

fn reads(s: &Snapshot) -> Vec<String> {
    let p = "p_fixture";
    let cases = [
        ("get_outline", json!({"projectId":p}), false),
        (
            "get_outline",
            json!({"projectId":p,"limit":2,"depth":1}),
            false,
        ),
        ("get_outline", json!({"projectId":p,"depth":0}), false),
        (
            "get_outline",
            json!({"projectId":p,"nodeId":"frame"}),
            false,
        ),
        (
            "get_outline",
            json!({"projectId":p,"frameId":"frame-b"}),
            false,
        ),
        (
            "get_subtree",
            json!({"projectId":p,"nodeId":"frame","properties":["name","text"]}),
            false,
        ),
        (
            "get_subtree",
            json!({"projectId":p,"nodeId":"page-a","depth":1,"limit":3}),
            false,
        ),
        (
            "get_design_context",
            json!({"projectId":p,"frameId":"frame"}),
            false,
        ),
        (
            "get_design_context",
            json!({"projectId":p,"frameId":"frame","theme":"night","limit":2,"properties":["fill"]}),
            false,
        ),
        (
            "get_design_context",
            json!({"projectId":p,"frameId":"frame","budget":2048}),
            true,
        ),
        (
            "get_design_context",
            json!({"projectId":p,"frameId":"frame-b"}),
            true,
        ),
    ];
    cases
        .into_iter()
        .flat_map(|(tool, args, bounds)| pages(s, tool, args, bounds))
        .collect()
}

#[test]
fn outline_subtree_and_context_bytes_match_golden() {
    let s = fixture();
    let actual = reads(&s).join("\n") + "\n";
    if std::env::var_os("OPENCANVAS_WRITE_GOLDEN").is_some() {
        std::fs::write(GOLDEN, &actual).unwrap();
    }
    let golden = std::fs::read_to_string(GOLDEN).unwrap();
    assert_eq!(actual, golden);
}

/// The pre-index algorithm, kept as the reference for sibling order.
fn legacy<'a>(all: &'a [Node], node: &'a Node, depth: usize, out: &mut Vec<(&'a Node, usize)>) {
    out.push((node, depth));
    let mut children: Vec<_> = all
        .iter()
        .filter(|n| n.parent_id.as_deref() == Some(&node.id))
        .collect();
    children.sort_by_key(|n| n.sort_key);
    for child in children {
        legacy(all, child, depth + 1, out);
    }
}

#[test]
fn index_preorder_matches_legacy_visit_for_every_scope() {
    let s = fixture();
    let index = TreeIndex::new(&s.nodes);
    let ids = |list: Vec<(&Node, usize)>| -> Vec<(String, usize)> {
        list.into_iter().map(|(n, d)| (n.id.clone(), d)).collect()
    };
    let mut roots: Vec<_> = s.nodes.iter().filter(|n| n.parent_id.is_none()).collect();
    roots.sort_by_key(|n| n.sort_key);
    let mut expected = Vec::new();
    for root in roots {
        legacy(&s.nodes, root, 0, &mut expected);
    }
    assert_eq!(ids(index.preorder(None)), ids(expected));
    for node in &s.nodes {
        let mut expected = Vec::new();
        legacy(&s.nodes, node, 0, &mut expected);
        assert_eq!(ids(index.preorder(Some(&node.id))), ids(expected));
    }
    let chain: Vec<_> = index
        .ancestors("deep-text", None)
        .iter()
        .map(|n| n.id.as_str())
        .collect();
    assert_eq!(chain, ["page-a", "frame", "a-card", "deep"]);
    let scoped: Vec<_> = index
        .ancestors("deep-text", Some("frame"))
        .iter()
        .map(|n| n.id.as_str())
        .collect();
    assert_eq!(scoped, ["frame", "a-card", "deep"]);
}
